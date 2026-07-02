# Visual projection: showing the deep world, a scoped proposal

This is a design proposal for owner sign-off, not consolidated design. It answers the question "how do we visually show all the physics, anatomy, materials, and the rest, and do we do it with procgen pixel art?" It proposes a visual-projection substrate that derives appearance from the canonical data the sim already computes, rather than authoring it, and it records the first slice already built (physics-derived terrain colour in the viewer).

## The thesis: derive appearance, never author it

Hand-drawn sprites for creatures, materials, and wounds would be the one thing the design forbids: an authored template (Principle 8). You cannot draw a sprite for every emergent morphology or material, and drawing them would impose appearance rather than letting it arise. So appearance follows the same rule as everything else: it is a projection of the canonical data, computed, not painted.

The lucky part is that the sim already computes what things look like. The chem-optics floor carries `reflectance`, `albedo`, `emissivity`, and `absorption`, the same optical axes Beer-Lambert and Snell use for the light physics. Materials are `Substance` vectors over those axes; anatomy is a per-part body graph with tissue composition; temperature, wetness, and wounds are live state. A material's colour is a read of its optical vector, a hot thing glows from its thermal state, and a creature's shape is its part graph. We render the physics the sim is already doing.

One freedom makes this cheap. The view is a consumer of canonical state (Principle 10: the sim runs identically whether or not it is observed), so the renderer is not on the canonical path. It can use floats, GPU shaders, and autotune freely, none of the bit-identity discipline the canonical engine enforces applies to pixels. Appearance can be as rich as we like.

## The substrate: a data-driven visual projection

A `VisualProjection` registry, a sibling of the value, semantic, and institution substrates: the mechanism is fixed Rust, the membership is data and grows with the world (Principle 11). It maps a canonical fact to a visual primitive:

- a physics or optical axis, or a tissue-composition axis, to a colour contribution;
- an anatomy part kind to a silhouette primitive (a tapered segment, a mass, a plate);
- a state axis (temperature, wetness, a wound, a fluid) to an overlay.

Because the mapping is data, a new substance, organ, or axis gets a look with no new code, exactly as a new value kind gets a distance in the value substrate.

## The three projections

**Material to palette.** Map the optical axes to colour: `reflectance` and `albedo` to base hue and value, `absorption` to saturation and opacity, `emissivity` times `temperature` to an additive blackbody glow. Terrain is the same idea one level up: a tile's own `elevation`, `moisture`, and `temperature` fields (the physical quantities worldgen computed) map to colour. This is the slice already built, `physics_terrain_color` in `crates/viewer/src/render.rs`.

**Anatomy to silhouette.** The per-part body graph gives parts, their sizes, and connectivity. A procedural body-plan renderer lays them out (a limb is a tapered segment, a torso a mass), sizes them by the anatomy vector, and mirrors across the symmetry axis, the classic symmetric-sprite technique but seeded by the graph rather than pure noise, so a six-legged armoured thing and a bipedal one both fall out of their own morphology. Each part is coloured by its tissue composition through the material projection.

**State to overlay.** Wounds and fluids (R-FLUID) as blood and scarring, wetness as a sheen, cold as desaturation, heat as glow, all read live from the being's state.

## The pipeline and the pixel-art question

Yes, procgen pixel art is the right richer view, and it is a projection, not an art asset. The pipeline for a being: seed from the entity's canonical id or genome so it is reproducible; build the silhouette from the part graph; colour each part through the material projection; apply the state overlays; rasterize to a small grid (16x16 or 32x32); shade from a fixed light direction; add the dark outline and a little ordered dithering that make pixel art read.

Caching answers "PNG or live": because appearance is a pure function of the appearance-relevant state, render a sprite once per (genome plus state-bucket), key it by a content hash of that state, atlas it, and evict by an LRU. The GPU stood up this session can batch-rasterize the atlas, and since the view is non-canonical it does so with ordinary float shaders.

## The two-tier view

Keep the glyph view (Part 14, in `crates/viewer`) as the thin, dependable default, truest to the Dwarf Fortress and Songs of Syx spirit, and layer the procgen sprite view on top as the richer optional projection. The glyph view ships; the sprite view is an increment invested in over time.

## Reserved values, surfaced with basis

The visual projection carries aesthetic calls, not physics, so they are the owner's to set, surfaced not fabricated. Each is a palette anchor with a basis:

- the terrain palette endpoints (sea level, the tan and green lowland ends, the snow and ochre temperature tints, the rock highland end), basis: the look the owner wants for a world at those field values, an aesthetic target the prototype's defaults stand in for;
- the material palette mapping (how an optical `reflectance` or `albedo` value maps to a screen colour), basis: a pleasing rendering of the physics, since a physical reflectance is not automatically a pleasant colour;
- the blackbody glow curve (how `emissivity` times `temperature` becomes an emissive tint), basis: the temperature at which a thing visibly glows, tied to the thermal floor's ranges;
- the sprite grid size, outline weight, and dither strength, basis: the pixel-art density the owner wants.

## The honest limits

Pure procgen sprites are hard to make look good; the failure mode is "everything is a noise blob," which is why the glyph view stays the floor and the sprite view is an increment. A physical reflectance is not a pleasing palette by default, so the optical-to-colour mapping needs a tuned projection rather than a raw read. Arbitrary emergent morphologies, not just humanoids, are the real difficulty in the silhouette renderer: the part graph gives the structure, drawing it well is the work. And the terrain slice reads the worldgen fields, which are a development fixture until the owner sets the worldgen calibration, so its look will shift when those land.

## Build order

1. Material to colour, the terrain slice: done, `physics_terrain_color` derives terrain colour from the tile's elevation, moisture, and temperature, blended with a light biome accent for identity, proven by `physics_terrain_colour_reflects_the_fields`.
2. The `VisualProjection` registry proper, promoting the terrain and material mappings into the data-driven substrate, and the material palette from the optical axes for substances.
3. The anatomy silhouette renderer over the part graph, with the material projection colouring parts.
4. The sprite atlas: caching by content hash, GPU rasterization, and the state overlays.

## What this proposal asks

Sign-off on the substrate shape (a data-driven `VisualProjection` registry deriving appearance from canonical physics, material, and anatomy data, with the renderer a non-canonical consumer), on the two-tier glyph-then-sprite view, and on the build order. The reserved palette values stay yours to set. The terrain slice is already in the viewer as a demonstration; the registry and the anatomy silhouette are the next builds on sign-off.
