import type { Plugin } from "@opencode-ai/plugin"

export const CargoCheckOnEdit: Plugin = async ({ $ }) => {
  return {
    "file.edited": async ({ path }) => {
      if (!path.endsWith(".rs") && !path.endsWith("Cargo.toml")) return

      try {
        const result = await $`cargo check --all-targets --message-format=short`.text()
        if (result.includes("error")) {
          console.warn(`[guardrail] cargo check errors after editing ${path}:\n${result}`)
        }
      } catch {
        // cargo may not be available; degrade gracefully
      }
    },
  }
}
