import { ReactNode, useState, useRef, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

// ============================================================================
// Tooltip Component
// Informational popover on hover
// ============================================================================

export interface TooltipProps {
  content: string | ReactNode;
  children: ReactNode;
  placement?: 'top' | 'bottom' | 'left' | 'right';
  delay?: number;
  className?: string;
  disabled?: boolean;
}

export function Tooltip({
  content,
  children,
  placement = 'top',
  delay = 200,
  className = '',
  disabled = false,
}: TooltipProps) {
  const [isVisible, setIsVisible] = useState(false);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const timeoutRef = useRef<NodeJS.Timeout>();
  const triggerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  const handleMouseEnter = () => {
    if (disabled) return;

    timeoutRef.current = setTimeout(() => {
      if (triggerRef.current) {
        const rect = triggerRef.current.getBoundingClientRect();
        calculatePosition(rect);
        setIsVisible(true);
      }
    }, delay);
  };

  const handleMouseLeave = () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    setIsVisible(false);
  };

  const calculatePosition = (rect: DOMRect) => {
    const offset = 8;
    let x = 0;
    let y = 0;

    switch (placement) {
      case 'top':
        x = rect.left + rect.width / 2;
        y = rect.top - offset;
        break;
      case 'bottom':
        x = rect.left + rect.width / 2;
        y = rect.bottom + offset;
        break;
      case 'left':
        x = rect.left - offset;
        y = rect.top + rect.height / 2;
        break;
      case 'right':
        x = rect.right + offset;
        y = rect.top + rect.height / 2;
        break;
    }

    setPosition({ x, y });
  };

  const tooltipVariants = {
    hidden: { opacity: 0, scale: 0.95 },
    visible: { opacity: 1, scale: 1 },
  };

  const arrowPosition = {
    top: 'bottom-[-4px] left-1/2 -translate-x-1/2 border-t-dark-700',
    bottom: 'top-[-4px] left-1/2 -translate-x-1/2 border-b-dark-700',
    left: 'right-[-4px] top-1/2 -translate-y-1/2 border-l-dark-700',
    right: 'left-[-4px] top-1/2 -translate-y-1/2 border-r-dark-700',
  };

  const tooltipPosition = {
    top: 'bottom-full mb-2 left-1/2 -translate-x-1/2',
    bottom: 'top-full mt-2 left-1/2 -translate-x-1/2',
    left: 'right-full mr-2 top-1/2 -translate-y-1/2',
    right: 'left-full ml-2 top-1/2 -translate-y-1/2',
  };

  return (
    <div
      ref={triggerRef}
      className="relative inline-block"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {children}

      <AnimatePresence>
        {isVisible && !disabled && (
          <motion.div
            initial="hidden"
            animate="visible"
            exit="hidden"
            variants={tooltipVariants}
            transition={{ duration: 0.15 }}
            className={`absolute z-50 ${tooltipPosition[placement]} ${className}`}
          >
            <div className="relative bg-dark-700 border border-dark-600 text-dark-100 text-sm px-3 py-2 rounded-lg shadow-xl max-w-xs whitespace-nowrap">
              {content}
              <div
                className={`absolute w-0 h-0 border-4 border-transparent ${arrowPosition[placement]}`}
              />
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// Simple info tooltip icon
export interface InfoTooltipProps {
  content: string | ReactNode;
  placement?: 'top' | 'bottom' | 'left' | 'right';
}

export function InfoTooltip({ content, placement = 'top' }: InfoTooltipProps) {
  return (
    <Tooltip content={content} placement={placement}>
      <span className="inline-flex items-center justify-center w-4 h-4 text-xs text-dark-400 hover:text-dark-200 cursor-help border border-dark-600 rounded-full transition-colors">
        i
      </span>
    </Tooltip>
  );
}
