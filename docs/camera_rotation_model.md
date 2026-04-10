# Camera Rotation Model

This document explains how the FourDeers camera handles 4D orientation, how it relates to the standard mathematical representation, and why the controls work the way they do.

## 1. The Standard Model: Spin(4)

Every 4D rotation can be encoded as a pair of unit quaternions `(q_L, q_R)` via the double cover map `Spin(4) -> SO(4)`. A 4D vector `v` (encoded as a quaternion `v = w + xi + yj + zk`) is rotated as:

```
v' = q_L * v * q_R^{-1}
```

This is the **standard decomposition**: `q_L` and `q_R` each act from one side, and every element of `SO(4)` can be uniquely represented this way (up to a shared sign).

In this model, the two quaternions don't individually "mean" anything geometric — they are just the two factors of a double-sided action. The actual rotation is the combination.

The code stores this pair in `Rotation4D` with fields named `q_left` and `q_right`, which are the mathematically standard names.

## 2. The Camera's Semantic Mapping

The camera assigns **specific semantic roles** to these two quaternion slots:

| `Rotation4D` field | Camera role | What it controls |
|---|---|---|
| `q_left` | **Look** | In-slice orientation: yaw/pitch of the camera within the current 3D slice |
| `q_right` | **Tilt** | Slice orientation: how the 3D slice itself is rotated in 4D (XW/YW planes) |

This is **not** the standard way to interpret `(q_L, q_R)`. In the standard model, both quaternions contribute symmetrically to the final rotation. Here, the camera treats them as independent control axes:

- Mouse drag changes **look** (3D FPS-style yaw/pitch, stored in `q_left`)
- 4D tilt controls change **tilt** (XW/YW rotation, stored in `q_right`)

Both still participate in the full rotation `q_L * v * q_R^{-1}` for rendering. But for **movement**, the camera uses them through a decoupled pipeline rather than the standard rotation basis.

Note: **look** and **tilt** are both *rotations*. **Movement** (translation along an axis) is a separate thing — the camera derives its movement directions from the look and tilt quaternions, but the quaternions themselves are not movement.

## 3. The Two-Stage Movement Pipeline

Movement axes are **not** derived from the standard rotation basis. The camera computes them through a two-stage pipeline that keeps in-slice movement and 4D movement independent.

### Standard rotation basis

The standard basis vectors of the rotation `R(v) = q_L * v * q_R^{-1}` are:

```
e_i' = q_L * e_i * q_R^{-1}
```

These are what `Rotation4D::basis_vectors()` returns. They are used for rendering: transforming world vertices into camera space.

### Camera movement axes

The camera computes its **movement** axes through a different pipeline:

```
Step 1: Look direction (3D)
    forward3 = q_L * (0,0,1) * q_L^{-1}     // standard 3D conjugation
    right3   = q_L * (1,0,0) * q_L^{-1}
    up3      = q_L * (0,1,0) * q_L^{-1}

Step 2: Project to 4D through tilt
    forward4 = project_via_tilt(forward3)     // uses (I, q_R) basis only
    right4   = project_via_tilt(right3)
    up4      = project_via_tilt(up3)

Step 3: Slice-normal movement
    w_axis   = (I, q_R).basis_w()             // W-column of tilt-only rotation
    kata     = +w_axis                         // translate along slice normal
    ana      = -w_axis                         // translate against slice normal
```

### Why two stages?

The key difference from the standard basis is **Step 2**: the 3D look directions are projected through the **tilt-only** rotation `(identity, q_R)`, not through the full `(q_L, q_R)` rotation.

This means:

- **Forward/Backward/Left/Right/Up/Down** movement follows what you **see** in the current slice. If you're looking right and press forward, you move in the direction you're looking — projected into 4D by how the slice is tilted.
- **Kata/Ana** movement is **translation** along the slice normal. The slice normal is derived from the tilt quaternion (`(I, q_R).basis_w()`), so kata/ana depends only on tilt, not on look direction.

If the camera used the standard basis for movement, changing your 3D look direction would alter your kata/ana direction (because the full basis interleaves look and tilt). The two-stage pipeline keeps them independent.

### Mathematical formulation

The camera computes each in-slice movement axis in two steps:

1. Derive a 3D direction from the look quaternion via standard conjugation:
   `forward3 = q_L * (0,0,1) * q_L^{-1}`

2. Project into 4D using the tilt-only basis. Concretely, `project_3d_to_4d_with_basis`
   embeds the 3D vector as `[x, y, z, 0]` and multiplies by the 3x4 submatrix of the
   `(I, q_R)` rotation basis:

```
forward_move[i] = sum_j(forward3[j] * tilt_basis[j][i])   for i in 0..4
```

where `tilt_basis[j]` is the j-th basis vector of `(I, q_R)` — the result of applying the
tilt-only rotation to the j-th standard basis vector. Equivalently: compute the 3D look
direction, embed it in 4D as `[x, y, z, 0]`, then apply the tilt rotation to map it into
world 4D space.

For kata/ana, the movement axis is simply the W-column of `(I, q_R)`:
```
kata_axis = (I, q_R).rotate_point([0,0,0,1])
```

This is the slice normal — the direction orthogonal to the tilted 3D slice.

## 4. Why `rotate_4d` Extracts `.q_left()` From Plane Rotations

This is a common source of confusion. The code in `Camera::rotate_4d`:

```rust
let tilt_xw = Rotation4D::from_plane_angle(RotationPlane::XW, angle);
let new_q_right = *tilt_xw.q_left() * *self.rotation_4d.q_right() * *tilt_yw.q_left();
```

Why does it extract `.q_left()` to update the tilt quaternion (stored in `q_right`)?

`Rotation4D::from_plane_angle` constructs rotations differently depending on the plane:

- **XY/XZ/YZ planes**: `q_left = q_right = q(angle)` — standard 3D conjugation, leaves W unchanged.
- **XW/YW/ZW planes**: `q_left = q(angle)`, `q_right = q(angle)^{-1}`.

So for an XW rotation by angle `a`, `from_plane_angle` creates the pair `(q, q^{-1})`. The full rotation action is:
```
v' = q * v * (q^{-1})^{-1} = q * v * q
```

This correctly mixes spatial axes with W. Now, the camera accumulates tilt into `q_right`. It extracts the raw quaternion `q` from `tilt_xw.q_left()` and composes it into `q_right`:
```
new_q_right = tilt_xw.q_left() * old_q_right * tilt_yw.q_left()
```

Later, when the full rotation is applied for rendering as `q_L * v * q_R^{-1}`, this accumulated `q_right` is inverted by the formula: the stored `q` acts as `q^{-1}`. The net effect is the correct XW/YW rotation.

The naming collision (extracting `.q_left()` to store into `q_right`) is unfortunate but mechanically correct. The `q_left`/`q_right` names belong to the math layer; the camera's **look**/**tilt** roles are a separate semantic overlay.

## 5. Rendering vs. Movement: Two Uses of the Same Quaternions

The same stored pair `(q_left, q_right)` is used in two distinct ways:

### For rendering (transforming geometry)

```
v' = q_left * v * q_right^{-1}
```

This is the full standard rotation. Used in `Rotation4D::rotate_point`, `basis_vectors`, `to_matrix`, etc. For projecting 4D vertices into 3D screen space, the shared `CameraProjection` struct encapsulates the decomposition:

```
projection = CameraProjection::new(camera)              // or ::with_object_rotation for scene
(xyz, w)    = projection.project(vertex)                 // position: mat_4d * v - offset, then mat_3d * r.xyz
direction   = projection.project_direction(dir)          // direction: mat_4d * d, then mat_3d * r.xyz
```

Internally this computes:
```
mat_4d = (I, q_right)^{-1}.to_matrix()     // undo tilt: applies q_right^{-1} from the right only
mat_3d = q_left^{-1}.to_rotation_matrix()   // undo look: applies q_left^{-1} from the left only
```

Both the scene renderer and the map renderer use `CameraProjection` for this transformation.

### For movement (deriving translation directions)

```
// In-slice movement (Forward/Backward/Left/Right/Up/Down)
look_3d  = q_left * (standard 3D vectors) * q_left^{-1}     // 3D conjugation
move_4d  = tilt_basis_matrix * [look_3d.x, look_3d.y, look_3d.z, 0]  // project via (I, q_R)

// Slice-normal movement (Kata/Ana)
slice_normal = (I, q_R).basis_w()                             // W-column of tilt-only rotation
```

Look determines the in-slice movement direction (what direction "forward" means within the slice). Tilt determines how that 3D direction maps into 4D, and independently provides the slice-normal direction for kata/ana. The full rotation `q_left * v * q_right^{-1}` is only used for rendering.

## 6. Relationship to Isoclinic Decomposition

Every 4D rotation can be decomposed into two **isoclinic rotations** (rotations where all points rotate by the same angle):

```
v' = q_L * v * q_R^{-1}
   = (q_L * v) * q_R^{-1}
```

- Left isoclinic: `v -> q_L * v` (left multiplication only, `q_R = identity`)
- Right isoclinic: `v -> v * q_R^{-1}` (right multiplication only, `q_L = identity`)

The camera's control decomposition aligns with the isoclinic decomposition at the storage level:

- A pure look change (`rotate`) modifies `q_left` with `q_right` fixed — a left-isoclinic change.
- A pure tilt change (`rotate_4d`) modifies `q_right` with `q_left` fixed — a right-isoclinic change.

However, the camera's **movement pipeline** treats the two components asymmetrically: look directions are projected through tilt for in-slice→4D mapping, but slice-normal movement (kata/ana) does not depend on look. This asymmetry is why the movement axes differ from the standard rotation basis — the quaternions are stored as isoclinic factors but used asymmetrically for deriving translation directions.

## 7. Summary

| Aspect | Standard model | Camera model |
|---|---|---|
| Storage | `(q_L, q_R)` | Same — `q_left` = look, `q_right` = tilt |
| Rotation action | `q_L * v * q_R^{-1}` | Same — used for rendering |
| In-slice movement | Full rotation basis | Two-stage: look direction projected through tilt basis |
| Slice-normal movement | Full basis W-column | `(I, q_R)` W-column only (tilt-derived, look-independent) |
| Look control | Not separated | Modifies `q_left` only |
| Tilt control | Not separated | Modifies `q_right` only |

The camera is a **reparameterization for control ergonomics**: it uses the same mathematical representation as the standard model but interprets the two quaternion slots as independent control axes (look and tilt), and derives movement directions through a decoupled pipeline that preserves the independence of in-slice movement and slice-normal movement.
