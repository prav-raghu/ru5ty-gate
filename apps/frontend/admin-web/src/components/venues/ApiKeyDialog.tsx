import { toast } from "../../store/toast.store";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";

interface ApiKeyDialogProps {
    title: string;
    gatewayName: string;
    apiKey: string;
    onClose: () => void;
}

export function ApiKeyDialog({ title, gatewayName, apiKey, onClose }: ApiKeyDialogProps) {
    const copy = async (): Promise<void> => {
        try {
            await navigator.clipboard.writeText(apiKey);
            toast.success("API key copied");
        } catch {
            toast.error("Could not copy. Select the key and copy it manually.");
        }
    };

    return (
        <Modal title={title} onClose={onClose}>
            <p className="mb-3 text-sm text-foreground">
                This is the only time the key for <strong>{gatewayName}</strong> is shown. Put it in the agent&apos;s{" "}
                <code>central.api_key</code> setting now. If you lose it, rotate the key.
            </p>
            <code aria-label="API key" className="block break-all rounded-md border border-border bg-muted p-3 text-sm select-all">
                {apiKey}
            </code>
            <div className="mt-4 flex justify-end gap-2">
                <Button variant="secondary" onClick={() => void copy()}>
                    Copy key
                </Button>
                <Button onClick={onClose}>I have stored the key</Button>
            </div>
        </Modal>
    );
}
