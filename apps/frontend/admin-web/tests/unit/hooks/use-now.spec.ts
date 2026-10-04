import { act, renderHook } from "@testing-library/react";
import { useNow } from "../../../src/hooks/use-now";

describe("useNow", () => {
    beforeEach(() => {
        jest.useFakeTimers();
        jest.setSystemTime(new Date("2026-10-04T12:00:00Z"));
    });

    afterEach(() => {
        jest.useRealTimers();
    });

    it("returns the current time and ticks on the interval", () => {
        const { result } = renderHook(() => useNow(1000));
        const initial = result.current;

        act(() => {
            jest.advanceTimersByTime(1000);
        });

        expect(result.current).toBe(initial + 1000);
    });

    it("stops ticking after unmount", () => {
        const clear = jest.spyOn(globalThis, "clearInterval");
        const { unmount } = renderHook(() => useNow(1000));
        unmount();
        expect(clear).toHaveBeenCalled();
    });
});
