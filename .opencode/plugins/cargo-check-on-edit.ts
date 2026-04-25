import type { Plugin } from "@opencode-ai/plugin"

export const CargoCheckOnEdit: Plugin = async ({ $ }) => {
  return {
    event: async ({ event }) => {
      if (event.type !== "file.edited") return

      const path = event.properties.file
      if (!path.endsWith(".rs") && !path.endsWith("Cargo.toml")) return

      try {
        const result = await $`cargo check --all-targets --message-format=short`.throws(false).text()
        if (result.includes("error")) {
          console.warn(`[guardrail] cargo check errors after editing ${path}:\n${result}`)
        }
      } catch {
        // cargo may not be available; degrade gracefully
      }
    },
  }
}
