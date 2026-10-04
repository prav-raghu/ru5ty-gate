import { toast, useToastStore } from "../../../src/store/toast.store";

describe("toast store", () => {
    afterEach(() => {
        useToastStore.getState().clear();
    });

    it("pushes success and error toasts with unique ids", () => {
        toast.success("Saved");
        toast.error("Failed");

        const { toasts } = useToastStore.getState();
        expect(toasts.map((entry) => [entry.kind, entry.message])).toEqual([
            ["success", "Saved"],
            ["error", "Failed"],
        ]);
        expect(toasts[0].id).not.toBe(toasts[1].id);
    });

    it("dismisses a single toast", () => {
        toast.success("One");
        toast.success("Two");
        const [first] = useToastStore.getState().toasts;

        useToastStore.getState().dismiss(first.id);

        expect(useToastStore.getState().toasts.map((entry) => entry.message)).toEqual(["Two"]);
    });

    it("clears all toasts", () => {
        toast.error("One");
        useToastStore.getState().clear();
        expect(useToastStore.getState().toasts).toEqual([]);
    });
});
