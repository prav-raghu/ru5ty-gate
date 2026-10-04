import type { ReactNode } from "react";
import { NavLink, useNavigate } from "react-router-dom";
import { APP_NAME } from "../../constants/config";
import { ROUTES } from "../../constants/routes";
import { authService } from "../../services/auth.service";
import { useAuthStore } from "../../store/auth.store";
import { toast } from "../../store/toast.store";
import { cn } from "../../utils/cn";
import { Button } from "../ui/Button";
import { Breadcrumbs, type Crumb } from "./Breadcrumbs";

interface AppShellProps {
    breadcrumbs: Crumb[];
    children: ReactNode;
}

const NAV_ITEMS = [{ label: "Venues", to: ROUTES.VENUES }] as const;

export function AppShell({ breadcrumbs, children }: AppShellProps) {
    const user = useAuthStore((state) => state.user);
    const clearAuth = useAuthStore((state) => state.clearAuth);
    const navigate = useNavigate();

    const signOut = async (): Promise<void> => {
        try {
            await authService.logout();
        } catch {
            toast.error("Could not reach the server. You were signed out on this device only.");
        }
        clearAuth();
        navigate(ROUTES.LOGIN);
    };

    return (
        <div className="flex min-h-screen bg-background text-foreground">
            <aside className="hidden w-56 shrink-0 border-r border-border bg-card md:block">
                <div className="px-4 py-5 text-lg font-semibold">{APP_NAME}</div>
                <nav aria-label="Main" className="space-y-1 px-2">
                    {NAV_ITEMS.map((item) => (
                        <NavLink
                            key={item.to}
                            to={item.to}
                            className={({ isActive }) =>
                                cn(
                                    "block rounded-md px-3 py-2 text-sm font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                                    isActive
                                        ? "bg-accent text-accent-foreground"
                                        : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                                )
                            }
                        >
                            {item.label}
                        </NavLink>
                    ))}
                </nav>
            </aside>
            <div className="flex min-w-0 flex-1 flex-col">
                <header className="flex items-center justify-between border-b border-border bg-card px-4 py-3">
                    <span className="text-sm text-muted-foreground md:hidden">{APP_NAME}</span>
                    <div className="ml-auto flex items-center gap-3">
                        {user ? (
                            <span className="text-sm text-muted-foreground">
                                {user.username} · {user.role}
                            </span>
                        ) : null}
                        <Button variant="secondary" onClick={() => void signOut()}>
                            Sign out
                        </Button>
                    </div>
                </header>
                <main className="flex-1 p-4 md:p-6">
                    <Breadcrumbs items={breadcrumbs} />
                    {children}
                </main>
            </div>
        </div>
    );
}
