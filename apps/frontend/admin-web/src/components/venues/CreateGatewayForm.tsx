import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { gatewayCreateSchema, type GatewayCreateForm } from "../../utils/venue-validation";
import { Button } from "../ui/Button";
import { TextField } from "../ui/TextField";

interface CreateGatewayFormProps {
    submitting: boolean;
    onSubmit: (name: string) => void;
    onCancel: () => void;
}

export function CreateGatewayForm({ submitting, onSubmit, onCancel }: CreateGatewayFormProps) {
    const {
        register,
        handleSubmit,
        formState: { errors },
    } = useForm<GatewayCreateForm>({ resolver: zodResolver(gatewayCreateSchema), defaultValues: { name: "" } });

    return (
        <form onSubmit={handleSubmit((values) => onSubmit(values.name))} noValidate className="space-y-4">
            <TextField
                label="Gateway name"
                hint="For example the router's location, such as front-desk."
                error={errors.name?.message}
                {...register("name")}
            />
            <div className="flex justify-end gap-2">
                <Button variant="secondary" onClick={onCancel}>
                    Cancel
                </Button>
                <Button type="submit" loading={submitting}>
                    Register gateway
                </Button>
            </div>
        </form>
    );
}
