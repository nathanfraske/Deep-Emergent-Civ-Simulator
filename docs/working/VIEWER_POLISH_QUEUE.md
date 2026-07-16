# Viewer polish queue (owner run-feedback, 2026-07-15)

Collected viewer UX items to land in ONE coherent viewer pass AFTER R-YOUNG-TEMPERATURE clears the viewer (both touch `crates/viewer/src/main.rs`, so they must not run concurrently, and the render polish only matters once worlds are bumpy). All viewer-only, non-canon: the two run pins must hold.

1. ZOOM-LEVEL INDICATOR. Show the current zoom scale on the HUD (system map / planet / surface, or a distance readout), so the observer does not lose their bearings across the continuous zoom. Owner: "we need some kind of indication as to what level of zoom you are, otherwise it may get easy to lose your bearings."

2. PAUSE MUST FREEZE THE DAY-NIGHT CYCLE. When paused (space) and zoomed into a planet, the day-night rotation keeps advancing. The planet's rotation / sun-direction sweep is running on a clock that ignores the `PlaybackDriver` pause state; gate it by the same pause the orbits already respect, so pause freezes BOTH the orbit and the surface day-night. Owner: "when I pause and zoom into a planet, the day night cycle keeps going even paused."

3. PAN TO SEE THE STAR WHILE ZOOMED INTO A PLANET. While zoomed in on a planet the camera is locked and the star cannot be seen; let the camera pan or orbit so the observer can look around and see the star from the planet. Owner: "I cannot pan around to see the star either while zoomed in to the planet."

4. CONTINUOUS RELIEF + MATERIAL COLOUR RENDER (the "less jank" pass). The derived globe is a continuous materials-derived surface, not a fixed tile grid; the blocky look is the display resampling the coarse (derived) province field onto a viewer grid. Render it continuously: smooth the relief shading and material colour across the surface so there are no visible tile boundaries. This is presentation only, no physics change, and it is NOT a return to fractal noise. Real detail comes from more derived provinces (smaller convective cells) or a derived sub-province texture, never authored noise.

5. FINISH THE PAN-A-TILE "WHAT IT IS" READOUT (Stage 9 tail). The `p`-key provenance overlay already names a tile and shows the derivation grade of its values. Complete it into a clear "what this surface is" readout (the derived material, its colour basis, the elevation and its provenance), so panning the surface tells you what you are looking at.

Note: Stage 7 (retire fractal noise, derive tiles from the materials substrate) is already DONE on the derived globe. The remaining fractal noise is the legacy flat-map worldgen (`crates/world`), a separate system, not the capstone path.
