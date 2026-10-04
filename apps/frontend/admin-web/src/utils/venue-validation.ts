import { z } from "zod";

const CODE_PATTERN = /^[A-Za-z0-9_-]+$/;
const MIN_DURATION_SECS = 60;
const MAX_DURATION_SECS = 604_800;

const redirectUrl = z
    .string()
    .max(2048, "Redirect URL is too long")
    .refine((value) => value === "" || isHttpUrl(value), "Enter a valid http or https URL")
    .optional();

function isHttpUrl(value: string): boolean {
    try {
        const parsed = new URL(value);
        return parsed.protocol === "http:" || parsed.protocol === "https:";
    } catch {
        return false;
    }
}

const sessionDurationSecs = z
    .number({ error: "Enter the session length in seconds" })
    .int("Must be a whole number of seconds")
    .min(MIN_DURATION_SECS, "At least 60 seconds")
    .max(MAX_DURATION_SECS, "At most 7 days");

export const venueNameSchema = z.string().trim().min(1, "Name is required").max(120, "Name is too long (120 characters maximum)");

export const venueCreateSchema = z.object({
    code: z
        .string()
        .min(3, "Code must be at least 3 characters")
        .max(64, "Code is too long (64 characters maximum)")
        .regex(CODE_PATTERN, "Use letters, digits, '-' or '_' only"),
    name: venueNameSchema,
    sessionDurationSecs,
    redirectUrl,
    allowNewSessions: z.boolean(),
});

export const venueUpdateSchema = z.object({
    name: venueNameSchema,
    sessionDurationSecs,
    redirectUrl,
    allowNewSessions: z.boolean(),
});

export const gatewayCreateSchema = z.object({
    name: z.string().trim().min(1, "Name is required").max(80, "Name is too long (80 characters maximum)"),
});

export type VenueCreateForm = z.infer<typeof venueCreateSchema>;
export type VenueUpdateForm = z.infer<typeof venueUpdateSchema>;
export type GatewayCreateForm = z.infer<typeof gatewayCreateSchema>;
