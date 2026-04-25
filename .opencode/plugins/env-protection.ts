import type { Plugin } from "@opencode-ai/plugin"

const PROTECTED_PATTERNS = [
  /\.env/,
  /credentials/,
  /secrets?/,
  /\.htpasswd/,
  /id_rsa/,
  /\.pem$/,
  /\.key$/,
  /token/,
]

export const EnvProtection: Plugin = async () => {
  return {
    "tool.execute.before": async (input, output) => {
      const path = output.args.filePath ?? output.args.path
      if (["read", "write", "edit"].includes(input.tool) && path) {
        const path = String(output.args.filePath ?? output.args.path)
        for (const pattern of PROTECTED_PATTERNS) {
          if (pattern.test(path)) {
            throw new Error(
              `Access to potentially sensitive file denied: ${path}\n` +
              `This file may contain secrets, credentials, or environment variables. ` +
              `If you genuinely need to read this file, ask the user for explicit permission.`
            )
          }
        }
      }
    },
  }
}
