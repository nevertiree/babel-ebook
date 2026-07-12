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
