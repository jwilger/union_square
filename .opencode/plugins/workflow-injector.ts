import type { Plugin } from "@opencode-ai/plugin"

export const WorkflowInjector: Plugin = async () => {
  return {
    event: async ({ event }) => {
      if (event.type === "session.created") {
        console.info(
          "[guardrail] Union Square Workflow Reminders:\n" +
          "  1. NEVER use --no-verify when committing\n" +
          "  2. Follow the exact todo list structure (tests → impl → commit → push)\n" +
          "  3. All work tracked via GitHub Issues\n" +
          "  4. Use Conventional Commits\n" +
          "  5. Ask for help when stuck — don't take shortcuts"
        )
      }
    },
  }
}
