#!/usr/bin/env python3
"""
Generate static polytope vertex/edge data for all 6 regular convex 4-polytopes.

Outputs src/polytopes_data.rs with pure constant-data functions.
Run: python3 tools/generate_polytopes.py
"""

import math
import sys
from itertools import permutations, product

# ─── Constants ───────────────────────────────────────────────────────────────

PHI = (1.0 + math.sqrt(5.0)) / 2.0  # golden ratio ≈ 1.618
PHI_INV = 1.0 / PHI  # ≈ 0.618
PHI2 = PHI * PHI  # ≈ 2.618
PHI_INV2 = PHI_INV * PHI_INV  # ≈ 0.382
SQRT5 = math.sqrt(5.0)


def _permutation_sign(p):
    """Return +1 for even permutation, -1 for odd."""
    visited = [False] * len(p)
    sign = 1
    for i in range(len(p)):
        if visited[i]:
            continue
        cycle_len = 0
        j = i
        while not visited[j]:
            visited[j] = True
            j = p[j]
            cycle_len += 1
        if cycle_len % 2 == 0:
            sign = -sign
    return sign


# All 24 permutations of 4 elements
ALL_PERMS_4 = list(permutations(range(4)))

# The 12 even permutations of 4 elements (sign = +1)
EVEN_PERMS_4 = [p for p in ALL_PERMS_4 if _permutation_sign(p) == 1]


# ─── Helpers ─────────────────────────────────────────────────────────────────


def permuted(coords, perms):
    """Apply permutations to a 4-tuple, yielding unique coordinate sets."""
    seen = set()
    for p in perms:
        result = (coords[p[0]], coords[p[1]], coords[p[2]], coords[p[3]])
        if result not in seen:
            seen.add(result)
            yield result


def signed_permuted(base_abs, perms):
    """Generate all sign combinations for a 4-tuple of absolute values,
    then apply the given permutations, yielding unique results."""
    seen = set()
    for signs in product([1.0, -1.0], repeat=4):
        coords = tuple(s * a for s, a in zip(signs, base_abs))
        for p in perms:
            result = (coords[p[0]], coords[p[1]], coords[p[2]], coords[p[3]])
            if result not in seen:
                seen.add(result)
                yield result


def dist_sq(v1, v2):
    return sum((a - b) ** 2 for a, b in zip(v1, v2))


def find_edges(vertices, expected_edge_length_sq):
    """Find edges by distance-based detection with tolerance."""
    tol = 1e-6
    edges = []
    for i in range(len(vertices)):
        for j in range(i + 1, len(vertices)):
            d = dist_sq(vertices[i], vertices[j])
            if abs(d - expected_edge_length_sq) < tol:
                edges.append((i, j))
    return edges


# ─── Validation ──────────────────────────────────────────────────────────────


def validate(
    name, vertices, edges, expected_v, expected_e, expected_degree, expected_antipodal
):
    ok = True

    def check(cond, msg):
        nonlocal ok
        if cond:
            print(f"  {msg} \u2713", file=sys.stderr)
        else:
            print(f"  {msg} FAILED", file=sys.stderr)
            ok = False

    check(len(vertices) == expected_v, f"{expected_v} vertices (got {len(vertices)})")

    check(len(edges) == expected_e, f"{expected_e} edges (got {len(edges)})")

    centroid = [sum(v[k] for v in vertices) / len(vertices) for k in range(4)]
    check(all(abs(c) < 1e-10 for c in centroid), "centroid at origin")

    if edges:
        lengths = [dist_sq(vertices[i], vertices[j]) for i, j in edges]
        mean_l = sum(lengths) / len(lengths)
        check(
            all(abs(l - mean_l) < 1e-6 for l in lengths),
            f"uniform edge length sq {mean_l:.6f}",
        )

    degrees = [0] * len(vertices)
    for i, j in edges:
        degrees[i] += 1
        degrees[j] += 1
    check(
        all(d == expected_degree for d in degrees), f"vertex degree {expected_degree}"
    )

    antipodal = 0
    for i in range(len(vertices)):
        for j in range(i + 1, len(vertices)):
            if all(abs(vertices[i][k] + vertices[j][k]) < 1e-6 for k in range(4)):
                antipodal += 1
    check(
        antipodal == expected_antipodal,
        f"antipodal pairs {expected_antipodal} (got {antipodal})",
    )

    status = "\u2713" if ok else "FAILED"
    print(f"{name}: {status}\n", file=sys.stderr)
    return ok


# ─── 5-cell ──────────────────────────────────────────────────────────────────


def generate_5_cell():
    sqrt5 = math.sqrt(5.0)
    inv_sqrt5 = 1.0 / sqrt5
    four_inv_sqrt5 = 4.0 / sqrt5

    vertices = [
        (1.0, 1.0, 1.0, -inv_sqrt5),
        (1.0, -1.0, -1.0, -inv_sqrt5),
        (-1.0, 1.0, -1.0, -inv_sqrt5),
        (-1.0, -1.0, 1.0, -inv_sqrt5),
        (0.0, 0.0, 0.0, four_inv_sqrt5),
    ]

    # All pairs of 5 vertices = 10 edges
    edges = [(i, j) for i in range(5) for j in range(i + 1, 5)]

    return vertices, edges


# ─── 8-cell (tesseract) ─────────────────────────────────────────────────────


def generate_8_cell():
    # All 16 combinations of (±1, ±1, ±1, ±1)
    vertices = []
    for i in range(16):
        x = 1.0 if (i & 1) else -1.0
        y = 1.0 if (i & 2) else -1.0
        z = 1.0 if (i & 4) else -1.0
        w = 1.0 if (i & 8) else -1.0
        vertices.append((x, y, z, w))

    # Differ in exactly one coordinate (Hamming distance = 1)
    edges = []
    for i in range(16):
        for bit in range(4):
            j = i ^ (1 << bit)
            if i < j:
                edges.append((i, j))

    return vertices, edges


# ─── 16-cell ─────────────────────────────────────────────────────────────────


def generate_16_cell():
    vertices = []
    for axis in range(4):
        for sign in [1.0, -1.0]:
            v = [0.0, 0.0, 0.0, 0.0]
            v[axis] = sign
            vertices.append(tuple(v))

    # Connect vertices on different axes
    edges = []
    for i in range(8):
        for j in range(i + 1, 8):
            if (i // 2) != (j // 2):
                edges.append((i, j))

    return vertices, edges


# ─── 24-cell ─────────────────────────────────────────────────────────────────


def generate_24_cell():
    # Permutations of two non-zero coords from {±1}
    vertices = []
    for i in range(4):
        for j in range(i + 1, 4):
            for si in [1.0, -1.0]:
                for sj in [1.0, -1.0]:
                    v = [0.0, 0.0, 0.0, 0.0]
                    v[i] = si
                    v[j] = sj
                    vertices.append(tuple(v))

    # Edge length squared = 2
    edges = find_edges(vertices, 2.0)

    return vertices, edges


# ─── 600-cell ────────────────────────────────────────────────────────────────


def generate_600_cell():
    vertices = []

    # Group 1 (8): all permutations of (±1, 0, 0, 0)
    for coords in permuted((1.0, 0.0, 0.0, 0.0), ALL_PERMS_4):
        for signs in product([1.0, -1.0], repeat=4):
            v = tuple(s * c for s, c in zip(signs, coords))
            # only keep if exactly one nonzero coord
            if sum(1 for x in v if abs(x) > 0) == 1:
                vertices.append(v)
    vertices = list(set(vertices))

    # Group 2 (16): all sign combos of (±1/2, ±1/2, ±1/2, ±1/2)
    for signs in product([1.0, -1.0], repeat=4):
        vertices.append(tuple(s * 0.5 for s in signs))

    # Group 3 (96): even permutations of (±φ/2, ±1/2, ±1/(2φ), 0)
    base = (PHI / 2.0, 0.5, PHI_INV / 2.0, 0.0)
    for v in signed_permuted(base, EVEN_PERMS_4):
        vertices.append(v)

    vertices = list(dict.fromkeys(vertices))  # deduplicate preserving order

    # Edge length squared = 1/φ²
    edge_length_sq = PHI_INV**2
    edges = find_edges(vertices, edge_length_sq)

    return vertices, edges


# ─── 120-cell ────────────────────────────────────────────────────────────────


def generate_120_cell():
    # Using √8-radius coordinates (edge length = 3 - √5)
    vertices = []

    # Group 1 (24): all permutations of (0, 0, ±2, ±2)
    for v in signed_permuted((0.0, 0.0, 2.0, 2.0), ALL_PERMS_4):
        vertices.append(v)

    # Group 2 (64): all permutations of (±φ, ±φ, ±φ, ±φ⁻²)
    for v in signed_permuted((PHI, PHI, PHI, PHI_INV2), ALL_PERMS_4):
        vertices.append(v)

    # Group 3 (64): all permutations of (±1, ±1, ±1, ±√5)
    for v in signed_permuted((1.0, 1.0, 1.0, SQRT5), ALL_PERMS_4):
        vertices.append(v)

    # Group 4 (64): all permutations of (±φ⁻¹, ±φ⁻¹, ±φ⁻¹, ±φ²)
    for v in signed_permuted((PHI_INV, PHI_INV, PHI_INV, PHI2), ALL_PERMS_4):
        vertices.append(v)

    # Group 5 (96): even permutations of (0, ±φ⁻¹, ±φ, ±√5)
    for v in signed_permuted((0.0, PHI_INV, PHI, SQRT5), EVEN_PERMS_4):
        vertices.append(v)

    # Group 6 (96): even permutations of (0, ±φ⁻², ±1, ±φ²)
    for v in signed_permuted((0.0, PHI_INV2, 1.0, PHI2), EVEN_PERMS_4):
        vertices.append(v)

    # Group 7 (192): even permutations of (±φ⁻¹, ±1, ±φ, ±2)
    for v in signed_permuted((PHI_INV, 1.0, PHI, 2.0), EVEN_PERMS_4):
        vertices.append(v)

    vertices = list(dict.fromkeys(vertices))  # deduplicate preserving order

    # Edge length = 3 - √5, so edge_length_sq = (3 - √5)² = 14 - 6√5
    edge_length_sq = (3.0 - SQRT5) ** 2
    edges = find_edges(vertices, edge_length_sq)

    return vertices, edges


# ─── Code generation ─────────────────────────────────────────────────────────


def format_float(f):
    """Format a float as a Rust literal, truncated to f32 precision."""
    if f == 0.0:
        return "0.0"
    s = f"{f:.8g}"
    if "." not in s and "e" not in s and "E" not in s:
        s += ".0"
    return s


def generate_rust_function(name, vertices, edges):
    """Generate a Rust create_X_cell function."""
    lines = []
    lines.append("#[allow(clippy::excessive_precision)]")
    lines.append(f"fn {name}() -> (Vec<nalgebra::Vector4<f32>>, Vec<u16>) {{")

    lines.append("    let vertices = vec![")
    for v in vertices:
        coords = ", ".join(format_float(c) for c in v)
        lines.append(f"        nalgebra::Vector4::new({coords}),")
    lines.append("    ];")

    lines.append("")
    lines.append("    let indices: Vec<u16> = vec![")
    # Pack edges as index pairs
    idx_parts = []
    for i, j in edges:
        idx_parts.append(f"{i}, {j}")
    # Write in chunks of 5 pairs per line for readability
    chunk_size = 5
    for k in range(0, len(idx_parts), chunk_size):
        chunk = idx_parts[k : k + chunk_size]
        lines.append(f"        {', '.join(chunk)},")
    lines.append("    ];")

    lines.append("")
    lines.append("    (vertices, indices)")
    lines.append("}")
    return "\n".join(lines)


# ─── Main ────────────────────────────────────────────────────────────────────


def generate_code(polytopes, results):
    func_names = {
        "5-cell": "create_5_cell",
        "8-cell": "create_8_cell",
        "16-cell": "create_16_cell",
        "24-cell": "create_24_cell",
        "600-cell": "create_600_cell",
        "120-cell": "create_120_cell",
    }

    lines = [
        "// @generated by tools/generate_polytopes.py — DO NOT EDIT",
        "",
    ]

    for name, _, _, _, _, _ in polytopes:
        vertices, edges = results[name]
        fn_name = func_names[name]
        lines.append(generate_rust_function(fn_name, vertices, edges))
        lines.append("")

    return "\n".join(lines)


def main():
    polytopes = [
        ("5-cell", generate_5_cell, 5, 10, 4, 0),
        ("8-cell", generate_8_cell, 16, 32, 4, 8),
        ("16-cell", generate_16_cell, 8, 24, 6, 4),
        ("24-cell", generate_24_cell, 24, 96, 8, 12),
        ("600-cell", generate_600_cell, 120, 720, 12, 60),
        ("120-cell", generate_120_cell, 600, 1200, 4, 300),
    ]

    all_ok = True
    results = {}

    for name, gen_fn, nv, ne, deg, anti in polytopes:
        print(f"Validating {name}...", file=sys.stderr)
        vertices, edges = gen_fn()
        ok = validate(name, vertices, edges, nv, ne, deg, anti)
        if not ok:
            all_ok = False
        results[name] = (vertices, edges)

    if not all_ok:
        print("VALIDATION FAILED — not generating output", file=sys.stderr)
        sys.exit(1)

    print("All validations passed. Generating Rust code...", file=sys.stderr)
    print(generate_code(polytopes, results))


if __name__ == "__main__":
    main()
