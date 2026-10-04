import { z } from "zod";

export const loginSchema = z.object({
    email: z.string().min(1, "Email is required").email("Invalid email address").max(254, "Email is too long"),
    password: z.string().min(8, "Password must be at least 8 characters").max(128, "Password is too long"),
});

export const mfaSchema = z.object({
    code: z.string().regex(/^\d{6}$/, "Enter the 6-digit code from your authenticator app"),
});

export type LoginForm = z.infer<typeof loginSchema>;
export type MfaForm = z.infer<typeof mfaSchema>;
