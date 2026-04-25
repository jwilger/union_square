import type { Plugin } from "@opencode-ai/plugin"

export const NoVerifyBlocker: Plugin = async () => {
  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool === "bash" && output.args.command) {
        const cmd = output.args.command as string
        if (/git\s+commit\b.*--no-verify/.test(cmd)) {
          throw new Error(
            "🚨 CRITICAL: The `--no-verify` flag is STRICTLY FORBIDDEN. " +
            "It bypasses pre-commit hooks that enforce code quality, formatting, linting, tests, and conventional commits. " +
            "If hooks are failing, fix the issues or ask for help. NEVER bypass checks."
          )
        }
      }
    },
  }
}
