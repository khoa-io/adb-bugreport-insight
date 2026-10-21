# Mermaid diagrams and validation

Mermaid is useful for non-UML diagrams such as flowcharts, timelines and explanatory graphs.
Use it when the notation fits the question and the intended Markdown viewer supports it.
Repository UML views use the [PlantUML/SVG workflow](03-diagrams.md#diagram-authoring-and-markdown-preview).
The current documentation checker rejects inline diagram fences; follow repository presentation rules before adding a Mermaid diagram here.

## Validation

Validate syntax with a renderer that supports the chosen diagram type, then inspect the actual layout in the intended viewer.
Check labels, relationships, clipping and readability; parser acceptance alone does not establish layout quality.
Keep diagrams scoped so labels remain readable at normal zoom.
A renderer that ignores direction or metadata cannot validate those properties; use the viewer's supported renderer when they matter.
