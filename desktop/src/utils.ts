/**
 * Generate a short, collision-resistant client-side identifier.
 *
 * The combination of a millisecond timestamp and a random suffix is sufficient
 * for UI keys and log/toast identifiers that only need uniqueness within the
 * current session.
 */
export function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

/**
 * Best-effort heuristic: does `model` (served by `provider`) support vision /
 * multimodal image input? Provider /models endpoints do not expose capability,
 * so this matches known vision model naming patterns. Unknown providers /
 * unmatched models default to true so we never hide a potentially-valid vision
 * model from the OCR picker; the user can always fall back to "custom".
 */
export function isVisionModel(provider: string, model: string): boolean {
  const m = model.toLowerCase();
  switch (provider) {
    case "openai":
      return (
        m.startsWith("gpt-4o") ||
        m.startsWith("gpt-4-turbo") ||
        m.startsWith("gpt-4.1") ||
        m.startsWith("gpt-4-vision") ||
        m.startsWith("o1") ||
        m.startsWith("o3") ||
        m.startsWith("o4") ||
        m.includes("vision")
      );
    case "anthropic":
      // All Claude 3 / 3.5 / 4 models accept image input.
      return m.startsWith("claude-3") || m.startsWith("claude-4");
    case "ollama":
      return (
        m.includes("llava") ||
        m.includes("vision") ||
        m.includes("minicpm-v") ||
        m.includes("vl")
      );
    case "deepseek":
      // deepseek-vl / deepseek-vl2 are vision; deepseek-chat / -reasoner are text.
      return m.includes("vl");
    default:
      // openai-compatible or unknown: don't hide anything.
      return true;
  }
}
