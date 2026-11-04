import { AiMagicIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "@/lib/utils";

/** Charlotte logo mark. */
export function BrandMark({ className }: { className?: string }) {
  return (
    <div className={cn("flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground", className)}>
      <HugeiconsIcon icon={AiMagicIcon} strokeWidth={2} className="size-4" />
    </div>
  );
}
