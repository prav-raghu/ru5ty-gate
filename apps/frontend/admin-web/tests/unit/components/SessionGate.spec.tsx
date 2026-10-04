const mockRestore = jest.fn();
jest.mock("../../../src/services/session-restore", () => ({ restoreSession: () => mockRestore() }));

import { render, screen } from "@testing-library/react";
import { SessionGate } from "../../../src/components/SessionGate";

describe("SessionGate", () => {
    it("shows a status skeleton until the session has been restored, then the children", async () => {
        let finish: () => void = () => undefined;
        mockRestore.mockReturnValue(new Promise<void>((resolve) => (finish = resolve)));

        render(
            <SessionGate>
                <p>App content</p>
            </SessionGate>,
        );

        expect(screen.getByRole("status", { name: "Restoring your session" })).toBeInTheDocument();
        expect(screen.queryByText("App content")).not.toBeInTheDocument();

        finish();

        expect(await screen.findByText("App content")).toBeInTheDocument();
    });

    it("renders the children even when restoring rejects", async () => {
        mockRestore.mockImplementation(async () => {
            throw new Error("boom");
        });

        render(
            <SessionGate>
                <p>App content</p>
            </SessionGate>,
        );

        expect(await screen.findByText("App content")).toBeInTheDocument();
    });
});
