# Menu-bar design assets

Source: corpora, `Examples/Menubar`, Figma node `49:1621`.
The exported SVG roots are retained unchanged; dimensions below are logical pixels.

| Asset | Design layer | Export dimensions | Rendered slot |
| --- | --- | --- | --- |
| tray.png | 50:4397 | 14 × 14 | Native template status-item image; dark export background converted to transparency |
| brand.svg | 49:4169 | 19 × 21.6553 | Header, original aspect ratio |
| crates.png | 50:4373 | 227 × 227 source | 16 × 16 crates.io icon |
| github.svg | 50:4390 | 16 × 15 | GitHub button, original masked group |
| mcp.svg | 49:4305 | 15 × 15 | 25 × 25 MCP disclosure badge |
| copy.svg | I49:4339;4355:5905 | 11 × 13 | Centered in the Copy row's 12 × 16 symbol slot |

The mask exports are retained as source references. The runtime GitHub and MCP
SVGs include their masks as exported from their complete groups. The tray keeps
version and server state dynamic; no desktop wallpaper or reference screenshot
is used as interface content.
