import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { venueCreateSchema, type VenueCreateForm } from "../../utils/venue-validation";
import { Button } from "../ui/Button";
import { TextField } from "../ui/TextField";

interface VenueFormProps {
    mode: "create" | "edit";
    defaultValues: VenueCreateForm;
    submitting: boolean;
    onSubmit: (values: VenueCreateForm) => void;
    onCancel: () => void;
}

export function VenueForm({ mode, defaultValues, submitting, onSubmit, onCancel }: VenueFormProps) {
    const schema = mode === "create" ? venueCreateSchema : venueCreateSchema.extend({ code: z.string() });
    const {
        register,
        handleSubmit,
        formState: { errors },
    } = useForm<VenueCreateForm>({ resolver: zodResolver(schema), defaultValues });

    return (
        <form onSubmit={handleSubmit(onSubmit)} noValidate className="space-y-4">
            {mode === "create" ? (
                <TextField
                    label="Code"
                    hint="The venue id the router agent is configured with. It cannot be changed later."
                    error={errors.code?.message}
                    {...register("code")}
                />
            ) : null}
            <TextField label="Name" error={errors.name?.message} {...register("name")} />
            <TextField
                label="Session length (seconds)"
                type="number"
                inputMode="numeric"
                hint="How long a client stays connected. Between 60 seconds and 7 days."
                error={errors.sessionDurationSecs?.message}
                {...register("sessionDurationSecs", { valueAsNumber: true })}
            />
            <TextField
                label="Redirect URL"
                type="url"
                hint="Where clients land after connecting. Leave empty to send them to the page they asked for."
                error={errors.redirectUrl?.message}
                {...register("redirectUrl")}
            />
            <label className="flex items-center gap-2 text-sm text-foreground">
                <input type="checkbox" className="size-4 accent-primary" {...register("allowNewSessions")} />
                Accept new sessions
            </label>
            <div className="flex justify-end gap-2 pt-2">
                <Button variant="secondary" onClick={onCancel}>
                    Cancel
                </Button>
                <Button type="submit" loading={submitting}>
                    {mode === "create" ? "Create venue" : "Save changes"}
                </Button>
            </div>
        </form>
    );
}
