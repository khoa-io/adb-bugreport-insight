# ADB Bugreport Insight

`bugreport-insight` is a command-line tool that give you insightful information from Android `adb bugreport` ZIP archives. It also provides a MCP server so that your AI assistant can read these insights and inspect the bugreport.

Example usage:
* `bugreport-insight BUGREPORT.zip` will generate local insight artifacts from the bugreport ZIP archive without extracting its contents.
* `bugreport-insight --mcp BUGREPORT.zip` will start a MCP server that your AI assistant can connect to.
