import type { Plugin } from "@opencode-ai/plugin"

export const AstGreponEdit: Plugin = async ({ $ }) => {
  return {
    event: async ({ event }) => {
      if (event.type !== "file.edited") return

      const path = event.properties.file
      if (!path.endsWith(".rs")) return

      try {
        const result = await $`ast-grep scan --filter no-unwrap-in-production --filter no-expect-in-production --filter no-panic-macro-production ${path}`
          .throws(false)
          .text()
        if (result.trim()) {
          console.warn(`[guardrail] ast-grep findings in ${path}:\n${result}`)
        }
      } catch {
        // ast-grep may not be installed or configured; degrade gracefully
      }
    },
  }
}
