import { AxiosError, AxiosHeaders, type AxiosResponse } from "axios";
import { errorMessage } from "../../../src/utils/api-error";

function axiosError(status: number | null, message?: string): AxiosError {
    const error = new AxiosError("failed");
    if (status !== null) {
        error.response = {
            status,
            data: message === undefined ? {} : { isSuccessful: false, message },
            statusText: "",
            headers: {},
            config: { headers: new AxiosHeaders() },
        } satisfies AxiosResponse;
    }
    return error;
}

describe("errorMessage", () => {
    it("prefers the server message", () => {
        expect(errorMessage(axiosError(409, "Venue code already exists"))).toBe("Venue code already exists");
    });

    it("explains a 403", () => {
        expect(errorMessage(axiosError(403))).toBe("You do not have permission to do that.");
    });

    it("explains a missing response", () => {
        expect(errorMessage(axiosError(null))).toBe("Cannot reach the server. Check your connection.");
    });

    it("falls back to the error message for plain errors", () => {
        expect(errorMessage(new Error("boom"))).toBe("boom");
    });

    it("falls back to a generic message otherwise", () => {
        expect(errorMessage("weird")).toBe("Something went wrong. Please try again.");
        expect(errorMessage(axiosError(500))).toBe("failed");
    });
});
