import { toast } from "sonner";

/** Runs an action and shows its error (if any) as a toast. Returns true on success. */
export async function attempt(fn: () => Promise<unknown>, success?: string): Promise<boolean> {
  try {
    await fn();
    if (success) toast.success(success);
    return true;
  } catch (e) {
    toast.error((e as Error).message);
    return false;
  }
}
