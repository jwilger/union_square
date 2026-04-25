import type { Plugin } from "@opencode-ai/plugin"

const CONVENTIONAL_COMMIT_REGEX =
  /^(build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)(\([a-z-]+\))?!?: .{1,72}$/

export const ConventionalCommitGuard: Plugin = async () => {
  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool === "bash" && output.args.command) {
        const cmd = output.args.command as string
        const match = cmd.match(/git\s+commit\s+.*-m\s+["']([^"']+)["']/)
        if (match) {
          const message = match[1]
          if (!CONVENTIONAL_COMMIT_REGEX.test(message)) {
            throw new Error(
              `Invalid commit message: "${message}"\n` +
              `Commit messages must follow Conventional Commits format:\n` +
              `  <type>[optional-scope]: <description>\n` +
              `Types: feat, fix, docs, style, refactor, test, chore, ci, build, perf, revert\n` +
              `Example: "feat(proxy): add Bedrock provider support"`
            )
          }
        }
      }
    },
  }
}
