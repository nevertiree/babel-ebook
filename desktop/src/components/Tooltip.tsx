import { useEffect, useId, useRef, useState } from "react";
import "./Tooltip.css";
import { createPortal } from "react-dom";

interface TooltipProps {
  content: string;
  children: React.ReactNode;
  placement?: "top" | "bottom" | "left" | "right";
}

export default function Tooltip({ content, children, placement = "top" }: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const triggerRef = useRef<HTMLSpanElement>(null);
  const tooltipId = useId();

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") setVisible(false);
    }
    if (visible) {
      window.addEventListener("keydown", handleKeyDown);
      return () => window.removeEventListener("keydown", handleKeyDown);
    }
  }, [visible]);

  return (
    <span
      ref={triggerRef}
      className="tooltip-trigger"
      aria-describedby={visible ? tooltipId : undefined}
      onMouseEnter={() => setVisible(true)}
      onMouseLeave={() => setVisible(false)}
      onFocus={() => setVisible(true)}
      onBlur={() => setVisible(false)}
    >
      {children}
      {visible &&
        createPortal(
          <TooltipBubble
            target={triggerRef.current}
            content={content}
            id={tooltipId}
            placement={placement}
          />,
          document.body
        )}
    </span>
  );
}

interface TooltipBubbleProps {
  target: HTMLElement | null;
  content: string;
  id: string;
  placement: TooltipProps["placement"];
}

function TooltipBubble({ target, content, id, placement }: TooltipBubbleProps) {
  const bubbleRef = useRef<HTMLDivElement>(null);
  const [style, setStyle] = useState<React.CSSProperties>({});

  useEffect(() => {
    if (!target || !bubbleRef.current) return;
    const rect = target.getBoundingClientRect();
    const bubble = bubbleRef.current.getBoundingClientRect();
    let top = 0;
    let left = 0;

    switch (placement) {
      case "top":
        top = rect.top - bubble.height - 8;
        left = rect.left + rect.width / 2 - bubble.width / 2;
        break;
      case "bottom":
        top = rect.bottom + 8;
        left = rect.left + rect.width / 2 - bubble.width / 2;
        break;
      case "left":
        top = rect.top + rect.height / 2 - bubble.height / 2;
        left = rect.left - bubble.width - 8;
        break;
      case "right":
        top = rect.top + rect.height / 2 - bubble.height / 2;
        left = rect.right + 8;
        break;
    }

    // Keep inside viewport.
    const padding = 8;
    left = Math.max(padding, Math.min(left, window.innerWidth - bubble.width - padding));
    top = Math.max(padding, Math.min(top, window.innerHeight - bubble.height - padding));

    setStyle({ top, left });
  }, [target, placement]);

  return (
    <div
      ref={bubbleRef}
      id={id}
      role="tooltip"
      className={`tooltip-bubble tooltip-bubble-${placement}`}
      style={style}
    >
      {content}
    </div>
  );
}
