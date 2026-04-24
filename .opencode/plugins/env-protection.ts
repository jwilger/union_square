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
      if (input.tool === "read" && output.args.filePath) {
        const path = output.args.filePath as string
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
