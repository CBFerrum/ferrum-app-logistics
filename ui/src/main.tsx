// Logistics' own interface at /logistics/: today's trips for dispatchers and a driver's stops, with
// every record in the records frame's lists and forms. A driver's stops open without a connection
// under the app's own worker (public/sw.js).

import { useEffect } from "react";
import { Link, useRouteError } from "react-router";
import { offlineEnv, routeBase, shell, sweep, t } from "@ferrum/client";
import { EmptyState, ErrorState } from "@ferrum/kit";
import { mountApp, RequireAuth, setFrameMode } from "@ferrum/records/frame";
import "./styles.css";

const page = (name: "FormView" | "ListView" | "ReportView") => () => import("./records").then((m) => ({ Component: m[name] }));

function RouteError() {
  return <ErrorState error={useRouteError()} onRetry={() => window.location.reload()} />;
}

function NotFound() {
  return (
    <EmptyState title={t("Page not found")}>
      <Link to="/">{t("Go to the trips")}</Link>
    </EmptyState>
  );
}

function Started() {
  useEffect(() => shell().ready(), []);
  return null;
}

// One that cannot register (a private window, an old browser) leaves the page working online.
if ("serviceWorker" in navigator) {
  void navigator.serviceWorker.register(`${routeBase("logistics")}sw.js`, { scope: routeBase("logistics") }).catch(() => {});
}
// What this device kept that the workspace's lease no longer covers goes (Ferrum docs/offline.md).
void sweep(offlineEnv()).catch(() => {});

setFrameMode("app", ["logistics"]);

mountApp({
  route: "logistics",
  catalogs: { ar: () => import("./locales/ar.json"), hi: () => import("./locales/hi.json") },
  routes: [
    {
      path: "/",
      element: (
        <>
          <Started />
          <RequireAuth />
        </>
      ),
      errorElement: <RouteError />,
      children: [
        { index: true, lazy: () => import("./Home").then((m) => ({ Component: m.Home })) },
        { path: "drive", lazy: () => import("./Drive").then((m) => ({ Component: m.Drive })) },
        { path: "e/:entity", lazy: page("ListView") },
        { path: "e/:entity/:id", lazy: page("FormView") },
        { path: "r/:name", lazy: page("ReportView") },
        { path: "*", element: <NotFound /> },
      ],
    },
  ],
});
