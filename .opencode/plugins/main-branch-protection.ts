import type { Plugin } from "@opencode-ai/plugin"

export const MainBranchProtection: Plugin = async () => {
  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool === "bash" && output.args.command) {
        const cmd = output.args.command as string
        if (/git\s+push\s+.*\b(main|master)\b/.test(cmd)) {
          throw new Error(
            "🚨 Direct pushes to main/master are FORBIDDEN.\n" +
            "All changes must go through a pull request.\n" +
            "Workflow: create feature branch → push branch → open PR → merge via PR."
          )
        }
      }
    },
  }
}
