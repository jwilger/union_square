import type { Plugin } from "@opencode-ai/plugin"

export const TodoStructureValidator: Plugin = async () => {
  return {
    "todo.updated": async ({ todos }) => {
      const items = todos.map((t: { content: string }) => t.content.toLowerCase())

      // Check for the standard workflow pattern
      const hasTests = items.some((c: string) => c.includes("test"))
      const hasImpl = items.some((c: string) => c.includes("implement") || c.includes("fix") || c.includes("refactor"))
      const hasCommit = items.some((c: string) => c.includes("commit"))
      const hasPush = items.some((c: string) => c.includes("push") || c.includes("pr"))

      if (items.length >= 3 && (!hasTests || !hasImpl || !hasCommit)) {
        console.warn(
          "[guardrail] Todo list may be missing standard workflow steps.\n" +
          "Expected structure:\n" +
          "  1. Write failing tests first\n" +
          "  2. Implementation/fix tasks\n" +
          "  3. Make a commit\n" +
          "  4. Push changes and update PR"
        )
      }
    },
  }
}
