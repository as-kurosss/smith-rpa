import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

/** Объединяет классы Tailwind без конфликтов (shadcn-утилита). */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
