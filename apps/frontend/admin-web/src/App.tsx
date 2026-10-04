import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { About } from "./pages/About";
import { Login } from "./pages/Login";
import { NotFound } from "./pages/NotFound";
import { VenueDetail } from "./pages/VenueDetail";
import { Venues } from "./pages/Venues";
import { ProtectedRoute } from "./components/ProtectedRoute";
import { ToastViewport } from "./components/ui/ToastViewport";
import { ROUTES } from "./constants/routes";

const queryClient = new QueryClient({
    defaultOptions: { queries: { refetchOnWindowFocus: false, retry: false } },
});

function App() {
    return (
        <QueryClientProvider client={queryClient}>
            <BrowserRouter>
                <Routes>
                    <Route path={ROUTES.LOGIN} element={<Login />} />
                    <Route path={ROUTES.HOME} element={<Navigate to={ROUTES.VENUES} replace />} />
                    <Route
                        path={ROUTES.VENUES}
                        element={
                            <ProtectedRoute>
                                <Venues />
                            </ProtectedRoute>
                        }
                    />
                    <Route
                        path={ROUTES.VENUE_DETAIL}
                        element={
                            <ProtectedRoute>
                                <VenueDetail />
                            </ProtectedRoute>
                        }
                    />
                    <Route
                        path="/about"
                        element={
                            <ProtectedRoute>
                                <About />
                            </ProtectedRoute>
                        }
                    />
                    <Route path="*" element={<NotFound />} />
                </Routes>
            </BrowserRouter>
            <ToastViewport />
        </QueryClientProvider>
    );
}

export default App;
