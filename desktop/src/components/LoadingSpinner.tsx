interface LoadingSpinnerProps {
  size?: number;
  className?: string;
}

export default function LoadingSpinner({ size = 16, className = "" }: LoadingSpinnerProps) {
  return (
    <span
      className={`loading-spinner ${className}`}
      style={{ width: size, height: size }}
      aria-hidden="true"
    />
  );
}
