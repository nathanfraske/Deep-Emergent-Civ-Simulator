# Render-strategy research: the three-band hybrid surface, and the cube-sphere warning

Status: research-catalog entry (researcher response, 2026-07-16, owner-relayed "for the research catalog"). Records the rendering strategy for a seeable planet surface and one structural warning the owner ruled apt to act on now while the render cache is young. Not a build spec, a design north star for the render slices that follow the crater-rows surface (slice 1, landed).

## The question

Are we rendering a sphere with painted pixels, and should it be an actual sphere with texture (real geometry), or is that overkill? Can we get sub-tile texturing so the eye sees stones?

## The answer: a three-band hybrid, allocating representation by on-screen feature size

Not a choice between painted and geometry. The standard planet-renderer answer allocates representation by how big a feature is on screen, one-to-one with the resolution ledger already built: every scale gets the cheapest representation that fools the eye at that scale.

- REAL GEOMETRY (displaced mesh) only for features big enough on screen to need silhouettes and parallax: crater rims when you stand at one, mountain profiles against the horizon, the flexural swells.
- NORMAL MAPPING for the middle band, a feature pixels-wide but not silhouette-scale: do not move vertices, tell the lighting the slope, and the eye reads it as three-dimensional because shading is most of what relief is.
- DETAIL MATERIALS for the bottom band (the "stones" instinct): high-frequency tiling albedo, normal, and roughness detail blended in near the camera, faded with distance. Individual stones are never objects, they are a statistical material, which is the sub-resolution texture-floor ruling wearing its render clothes.

## Why this is nearly free, not overkill

The hard problem in planet renderers is a height function you can query at any point and any level of detail, which is exactly `Sample(lat, lon, LOD)`, already ruled and building. It gives displacement by tessellating a tile and calling it per vertex. Better: because the height function is ANALYTIC (fields plus flexure Green's functions plus crater profiles plus the seeded spectral floor), the surface normal is the ANALYTIC GRADIENT of the same superposition, so normal maps need no baking and no finite differencing, evaluated as the derivative at query time. Derive-first in the renderer too. And the bottom-band detail material is not one generic rock: the written state knows what is underfoot, so a regime table selects it (regolith on airless cratered worlds, gravel where the hydrosphere carved, ropey basalt on volcanic provinces, fractured ice on shells). That is when deep zoom stops feeling painted, because the ground you land on is the ground the history wrote.

## Two calibration points (set expectations right)

- At GLOBE distance, full geometry buys nothing: 8 km of relief on a 6000 km radius is sub-pixel displacement, Earth from orbit is a smooth ball, and what reads as terrain at that range is hillshading (already built). The painted feel at globe view is not a defect, it is physics; the labeled exaggeration toggle is the honest tool for that view. The defect felt lives at ZOOM, which is where the geometry and detail bands earn their keep.
- COST: the whole planet meshed at stone resolution never happens, because the quadtree tessellates only what is in the view frustum, so you pay for what the camera looks at (the laziness-and-amortization mechanism applied to triangles).

## The structural warning (owner ruled APT to act on now, while the cache is young)

If the tiles are latitude-longitude, switch to a CUBE-SPHERE QUADTREE: six cube faces projected onto the sphere, each face a quadtree. Lat-lon tiles pinch at the poles, wreck the tile budget there, and every planet renderer that lived long enough migrated to cube-sphere; cheaper to be born on it. The slice-1 sample cache is currently a lat-lon grid (1440x720), so the migration is cheapest now, before the cache and its consumers entrench. Owner note: apt while we are here.

## Discipline (rides along untouched)

All of this is display-layer, one-way (canon to pixels, Principle 10). Normal maps are canon-consistent because they are derivatives of the canonical field. Detail materials are regime-conditioned class instances. Stones are seeded per world and location, so two views of the same spot always agree.

## Build order (researcher recommendation)

1. Analytic normals first (the biggest visual win per line of code, pure gradient math over the Sample superposition; the shading carries most of the relief).
2. Geometry tiles at zoom second (the quadtree tessellation calling Sample per vertex for the top band).
3. Detail materials keyed on the written state third (the regime table, the stones).

The cube-sphere migration is the substrate under all three and is the apt-now item; the stones the owner asked for are a detail-material slice away once the crater rows land (they have).
