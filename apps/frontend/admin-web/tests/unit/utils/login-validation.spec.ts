import { loginSchema, mfaSchema } from "../../../src/utils/login-validation";

describe("loginSchema", () => {
    it("accepts a valid email and password", () => {
        expect(loginSchema.safeParse({ email: "admin@test.com", password: "longenough" }).success).toBe(true);
    });

    it("rejects an empty email, a malformed email and a short password", () => {
        expect(loginSchema.safeParse({ email: "", password: "longenough" }).success).toBe(false);
        expect(loginSchema.safeParse({ email: "nope", password: "longenough" }).success).toBe(false);
        expect(loginSchema.safeParse({ email: "admin@test.com", password: "short" }).success).toBe(false);
    });
});

describe("mfaSchema", () => {
    it("accepts exactly six digits", () => {
        expect(mfaSchema.safeParse({ code: "123456" }).success).toBe(true);
    });

    it.each(["12345", "1234567", "12345a", ""])("rejects %j", (code) => {
        expect(mfaSchema.safeParse({ code }).success).toBe(false);
    });
});
