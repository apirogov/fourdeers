# FourDeers - Stereoscopic 4D Polytope Visualization

Hello fellow space pilot! So *you think* you understand the fourth dimension?

Then *prove* it - [go ahead](https://fourdeers.app.pirogov.de/) and jump right into 4D!

Find and explore all convex 4D polytopes using nothing but a map and a compass!

## Main Features

* uses [cross-eye stereo rendering](https://en.wikipedia.org/wiki/Stereoscopy) (because I do *not* have a VR headset, do *you*?)
* provides a "real" 4D camera with *full* 4D navigation (technically, an augmented 3D-slice camera)
* all spatial dimensions are treated on *equal* footing (unlike in many other 4D visualization tools)
* 4D minimap gadget with waypoints (which itself is a tesseract)
* compass gadget that helps understanding how to get to waypoints
* runs smoothly in the browser both on mobile and desktop

## Description

This repository was both a personal experiment in how far you can get with vibe coding
using current state-of-the-art models, and a fulfillment of a long-held personal dream - a
*proper* 4D viewer.

Because [this](https://upload.wikimedia.org/wikipedia/commons/5/55/8-cell-simple.gif) is
*not* a tesseract, this is a 3D projection of a tesseract with some automatic rotation, so
all the choices have already been made for you.

I always found that existing tools make the fourth dimension look more strange than it
really is. It is *just another coordinate* in your vectors, that's it. All weird
distortions and effects come from lossy projection to 3D. To get a more natural and
*intuitive* understanding, I wanted to come up with a tool to *explore 4D on its own
terms*.

In this app what you see is not some arbitrary projection, but a 3D slice that **you**
fully control. Within that slice you can move using an intuitive 3D camera, but you can
always **shift** and **tilt** your slice.

* in addition to the usual Forward/Backward, Up/Down, Left/Right there are two new directions: **Ana/Kata**
* in addition to 3D rotation of the camera *in* the current 3D slice, there is a **rotation *of* the current 3D slice** within the 4D space 
* with these additions, you can "x-ray" your way through the 4D scene and explore it any way you like

Now you would actually not see all that much if the "slice" was *really* mathematically
pure. What you get is actually a slab with controllable **thickness**, so more a piece of
bread than piece of paper. Think of the thickness as the distance how far you can "peek"
into ana and kata directions orthogonal to your current slice.

The color-coding (orange=positive, purple=negative) is used to indicate the distance from
the idealized slice center, whereas actual 3D information is transported by using your
eyes properly, i.e. giving you a stereo image for [cross-eye
viewing](https://stereoviewer.com/learn/cross-eye). I recommend to practice on some
standard stereo images if you have never done it before.

## Non-technical Explanation

If 4D space was a cube and what you see was a slice/layer of it (like a piece of paper / MRT image).

If normal movement supports directions *on* the piece of paper, then ana/kata would give
you "up" and "down" movement, which change the layer you are currently on. Without this,
you could never leave the paper. Colors show how far "up" or "down" an object is located.

If normal rotation rotates your facing angle *on* the piece of paper, then the 4D
rotations you get allow you to *tilt* the piece of paper itself. Without this, the
dimensions are not really treated equally, it would be like a pre-sliced bread where you
have no control over slice thickness or cutting angles.

## Usage

When you open the website, you will find a random 4D polytope in front of you.

Use cross-eye viewing to see the current projection in true 3D (or just look at one half
of the screen, if you do not care).

If you really brave, try to find some specific polytope using only the provided map.

If you get completely lost, open the compass, pick a waypoint and jump to it.

Mouse or finger tap controls work as follows.

Left half:

* **horizontal drag**: control slice thickness 
* **vertical drag**: control dichoptic color split 
  * *(you probably can ignore that, it was an experiment in exploiting [binocular rivalry](https://en.wikipedia.org/wiki/Binocular_rivalry))*

Right half: 

* **N/S/W/E**: Up/Down/Left/Right, **NE/SW**: Forward/Backward, **NW/SE**: Kata/Ana
* **center**: toggle rotation mode (camera 3D rotation / slice 4D rotation)
* **drag**: rotate 3D camera within slice / 3D slice within 4D scene

If you are on a desktop computer, the keyboard controls are explained in the menu.

## Development

### Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just (if not already installed)
cargo install just

# Install wasm-pack and setup WASM target
just setup
```

### Building & Running

```bash
# Build WASM (dev)
just wasm

# Serve on localhost:8888
just serve
# Open http://localhost:8888
```

### Standard Commands

```bash
cargo fmt                # Format code
cargo clippy             # Lint
cargo test               # Run all tests
cargo build [--release]  # Build native binary
```

### Extra Commands

| Command | Description |
|---------|-------------|
| `just wasm` | Build WASM (dev) |
| `just wasm-release` | Build WASM (release) |
| `just serve` | Serve on localhost:8888 |
| `just setup` | Add WASM target |

## Credits

Designed and [shamelessly vibe-coded](https://pirogov.de/blog/real-programmers/) by
[me](https://github.com/apirogov) on my phone, always when I would be otherwise wasting
time reading news or whatever.
Implemented by various models, mostly [GLM 5.1](https://docs.z.ai/guides/llm/glm-5.1).
I only wrote `AGENTS.md` and the majority of this `README.md`.
