import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { ROUTES } from "../constants/routes";
import { authService, type LoginData } from "../services/auth.service";
import { useAuthStore } from "../store/auth.store";
import { authTokenStore } from "../store/auth-token.store";
import { toast } from "../store/toast.store";
import { errorMessage } from "../utils/api-error";
import { loginSchema, mfaSchema, type LoginForm, type MfaForm } from "../utils/login-validation";
import { Button } from "../components/ui/Button";
import { TextField } from "../components/ui/TextField";

export function Login() {
    const navigate = useNavigate();
    const setAuth = useAuthStore((state) => state.setAuth);
    const [mfaToken, setMfaToken] = useState<string | null>(null);

    const credentials = useForm<LoginForm>({ resolver: zodResolver(loginSchema), defaultValues: { email: "", password: "" } });
    const mfa = useForm<MfaForm>({ resolver: zodResolver(mfaSchema), defaultValues: { code: "" } });

    const complete = async (data: LoginData): Promise<void> => {
        authTokenStore.setToken(data.authToken);
        const user = await authService.currentUser();
        setAuth(user, data.authToken);
        navigate(ROUTES.VENUES);
    };

    const submitCredentials = credentials.handleSubmit(async (values) => {
        try {
            const data = await authService.login(values.email, values.password);
            if (data.mfaRequired && data.mfaToken) {
                setMfaToken(data.mfaToken);
                return;
            }
            await complete(data);
        } catch (error) {
            authTokenStore.clearToken();
            toast.error(errorMessage(error));
        }
    });

    const submitMfa = mfa.handleSubmit(async (values) => {
        if (!mfaToken) {
            return;
        }
        try {
            await complete(await authService.verifyMfa(mfaToken, values.code));
        } catch (error) {
            authTokenStore.clearToken();
            toast.error(errorMessage(error));
        }
    });

    return (
        <div className="flex min-h-screen items-center justify-center bg-background p-4">
            <div className="w-full max-w-md rounded-lg border border-border bg-card p-8 shadow-sm">
                <h1 className="mb-2 text-2xl font-bold text-foreground">Admin Login</h1>
                <p className="mb-6 text-muted-foreground">
                    {mfaToken ? "Enter the code from your authenticator app" : "Sign in to access the admin dashboard"}
                </p>

                {mfaToken ? (
                    <form onSubmit={submitMfa} noValidate className="space-y-4">
                        <TextField
                            label="Authenticator code"
                            inputMode="numeric"
                            autoComplete="one-time-code"
                            error={mfa.formState.errors.code?.message}
                            {...mfa.register("code")}
                        />
                        <Button type="submit" className="w-full" loading={mfa.formState.isSubmitting}>
                            Verify
                        </Button>
                    </form>
                ) : (
                    <form onSubmit={submitCredentials} noValidate className="space-y-4">
                        <TextField
                            label="Email"
                            type="email"
                            autoComplete="username"
                            error={credentials.formState.errors.email?.message}
                            {...credentials.register("email")}
                        />
                        <TextField
                            label="Password"
                            type="password"
                            autoComplete="current-password"
                            error={credentials.formState.errors.password?.message}
                            {...credentials.register("password")}
                        />
                        <Button type="submit" className="w-full" loading={credentials.formState.isSubmitting}>
                            Sign in
                        </Button>
                    </form>
                )}
            </div>
        </div>
    );
}
