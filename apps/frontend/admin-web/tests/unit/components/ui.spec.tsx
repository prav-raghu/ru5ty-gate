import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Button } from "../../../src/components/ui/Button";
import { EmptyState } from "../../../src/components/ui/EmptyState";
import { ErrorState } from "../../../src/components/ui/ErrorState";
import { Modal } from "../../../src/components/ui/Modal";
import { Pagination } from "../../../src/components/ui/Pagination";
import { TableSkeleton } from "../../../src/components/ui/Skeleton";
import { TextField } from "../../../src/components/ui/TextField";
import { ToastViewport } from "../../../src/components/ui/ToastViewport";
import { toast, useToastStore } from "../../../src/store/toast.store";

describe("Button", () => {
    it("fires onClick", async () => {
        const onClick = jest.fn();
        render(<Button onClick={onClick}>Go</Button>);
        await userEvent.setup().click(screen.getByRole("button", { name: "Go" }));
        expect(onClick).toHaveBeenCalledTimes(1);
    });

    it("is disabled and busy while loading", () => {
        render(<Button loading>Save</Button>);
        const button = screen.getByRole("button", { name: "Working…" });
        expect(button).toBeDisabled();
        expect(button).toHaveAttribute("aria-busy", "true");
    });
});

describe("TextField", () => {
    it("links the error to the input and replaces the hint", () => {
        render(<TextField label="Name" hint="Helpful" error="Required" />);
        const input = screen.getByLabelText("Name");
        expect(input).toHaveAttribute("aria-invalid", "true");
        expect(screen.getByText("Required")).toBeInTheDocument();
        expect(screen.queryByText("Helpful")).not.toBeInTheDocument();
    });

    it("shows the hint when there is no error", () => {
        render(<TextField label="Name" hint="Helpful" />);
        expect(screen.getByText("Helpful")).toBeInTheDocument();
    });

    it("is valid without an error", () => {
        render(<TextField label="Name" />);
        expect(screen.getByLabelText("Name")).not.toHaveAttribute("aria-invalid", "true");
    });
});

describe("states", () => {
    it("ErrorState retries", async () => {
        const onRetry = jest.fn();
        render(<ErrorState message="Broke" onRetry={onRetry} />);
        expect(screen.getByText("Broke")).toBeInTheDocument();
        await userEvent.setup().click(screen.getByRole("button", { name: /retry|try again/i }));
        expect(onRetry).toHaveBeenCalled();
    });

    it("EmptyState shows title, description and action", () => {
        render(<EmptyState title="Nothing" description="Add one" action={<button type="button">Add</button>} />);
        expect(screen.getByText("Nothing")).toBeInTheDocument();
        expect(screen.getByText("Add one")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Add" })).toBeInTheDocument();
    });

    it("TableSkeleton exposes a status label", () => {
        render(<TableSkeleton label="Loading things" />);
        expect(screen.getByRole("status", { name: "Loading things" })).toBeInTheDocument();
    });
});

describe("Pagination", () => {
    it("disables previous on the first page and next without more data", () => {
        render(<Pagination page={1} hasNext={false} onPrevious={jest.fn()} onNext={jest.fn()} />);
        expect(screen.getByRole("button", { name: "Previous" })).toBeDisabled();
        expect(screen.getByRole("button", { name: "Next" })).toBeDisabled();
        expect(screen.getByText("Page 1")).toBeInTheDocument();
    });

    it("calls the handlers when enabled", async () => {
        const onPrevious = jest.fn();
        const onNext = jest.fn();
        render(<Pagination page={2} hasNext onPrevious={onPrevious} onNext={onNext} />);
        const user = userEvent.setup();
        await user.click(screen.getByRole("button", { name: "Previous" }));
        await user.click(screen.getByRole("button", { name: "Next" }));
        expect(onPrevious).toHaveBeenCalled();
        expect(onNext).toHaveBeenCalled();
    });
});

describe("Modal", () => {
    it("renders a labelled dialog and closes on Escape", async () => {
        const onClose = jest.fn();
        render(
            <Modal title="Edit thing" onClose={onClose}>
                <p>Body</p>
            </Modal>,
        );
        expect(screen.getByRole("dialog", { name: "Edit thing" })).toBeInTheDocument();
        await userEvent.setup().keyboard("{Escape}");
        expect(onClose).toHaveBeenCalledTimes(1);
    });

    it("keeps focus where the user put it when the parent re-renders", async () => {
        const user = userEvent.setup();
        const { rerender } = render(
            <Modal title="Form" onClose={() => undefined}>
                <input aria-label="Field" />
            </Modal>,
        );
        await user.click(screen.getByLabelText("Field"));
        rerender(
            <Modal title="Form" onClose={() => undefined}>
                <input aria-label="Field" />
            </Modal>,
        );
        expect(screen.getByLabelText("Field")).toHaveFocus();
    });
});

describe("ToastViewport", () => {
    afterEach(() => {
        jest.useRealTimers();
        act(() => useToastStore.getState().clear());
    });

    it("shows toasts and dismisses on click", async () => {
        render(<ToastViewport />);
        act(() => toast.error("Something failed"));
        expect(screen.getByText("Something failed")).toBeInTheDocument();
        await userEvent.setup().click(screen.getByRole("button", { name: "Dismiss notification" }));
        expect(screen.queryByText("Something failed")).not.toBeInTheDocument();
    });

    it("auto-dismisses after a delay", () => {
        jest.useFakeTimers();
        render(<ToastViewport />);
        act(() => toast.success("Saved"));
        expect(screen.getByText("Saved")).toBeInTheDocument();
        act(() => {
            jest.advanceTimersByTime(6000);
        });
        expect(screen.queryByText("Saved")).not.toBeInTheDocument();
    });
});
