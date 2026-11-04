"use client";
import { createContext, useContext } from "react";
import type { User } from "@/lib/api";

export const SessionContext = createContext<User | null>(null);

export function useUser(): User {
  const u = useContext(SessionContext);
  if (!u) throw new Error("useUser outside of the signed-in layout");
  return u;
}
