const KNOWN_PROVIDERS = ["deepseek", "openai", "anthropic", "ollama"] as const;
type KnownProvider = (typeof KNOWN_PROVIDERS)[number];

interface ProviderIconProps {
  provider: string;
  className?: string;
}

export default function ProviderIcon({ provider, className }: ProviderIconProps) {
  const known = KNOWN_PROVIDERS.includes(provider as KnownProvider) ? (provider as KnownProvider) : undefined;

  const commonProps = {
    width: 18,
    height: 18,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.8,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    className,
    "aria-hidden": true,
  };

  switch (known) {
    case "deepseek":
      return (
        <svg {...commonProps}>
          <circle cx="11" cy="11" r="8" />
          <path d="M21 21l-4.35-4.35" />
          <path d="M11 8v6" />
          <path d="M8 11h6" />
        </svg>
      );
    case "openai":
      return (
        <svg {...commonProps}>
          <path d="M12 2a10 10 0 0 1 10 10c0 5.523-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2z" />
          <path d="M12 7v5l3.5 2" />
        </svg>
      );
    case "anthropic":
      return (
        <svg {...commonProps}>
          <path d="M12 2l10.39 18H1.61L12 2z" />
          <path d="M12 8l3.5 6h-7L12 8z" />
        </svg>
      );
    case "ollama":
      return (
        <svg {...commonProps}>
          <path d="M4 14c0-4 3-7 8-7s8 3 8 7-3 7-8 7" />
          <path d="M4 14v4c0 2 1.5 3 3 3s3-1 3-3v-2" />
          <circle cx="9" cy="11" r="1" fill="currentColor" stroke="none" />
          <circle cx="15" cy="11" r="1" fill="currentColor" stroke="none" />
        </svg>
      );
    default:
      return null;
  }
}
