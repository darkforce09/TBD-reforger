# Interactive Tool Surfaces (`v2/apps/editor/input/tools`)

The browser half of the measure and selection tools: the ruler, the line-of-sight overlay and its
profile panel, the world-occluder seam, the viewshed job scheduler, and select-and-pick pointer
routing with its marquee.

**Depended on by:** the toolbelt's mode toolbar, which switches between them, and the canvas
pointer closures, which route presses to the active tool.

**Boundary:** the decidable half of every tool — its state machine, geometry and verdicts — lives
in `website_map_engine::editing::tools`. What belongs here is the DOM overlay, the transport and
the pointer routing that drive it.
