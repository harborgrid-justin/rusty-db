import { Suspense, lazy } from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import { AnimatePresence } from 'framer-motion';

import { MainLayout } from './components/layout/MainLayout';
import { LoadingScreen } from './components/common/LoadingScreen';
import { useAuth } from './hooks/useAuth';
import { ProtectedRoute } from './components/auth/ProtectedRoute';

// Lazy-loaded pages for code splitting
const Dashboard = lazy(() => import('./pages/Dashboard'));
const QueryEditor = lazy(() => import('./pages/QueryEditor'));
const TableManagement = lazy(() => import('./pages/TableManagement'));
const TableDetails = lazy(() => import('./pages/TableDetails'));
const UserManagement = lazy(() => import('./pages/UserManagement'));
const RoleManagement = lazy(() => import('./pages/RoleManagement'));
const Monitoring = lazy(() => import('./pages/Monitoring'));
const Sessions = lazy(() => import('./pages/Sessions'));
const SlowQueries = lazy(() => import('./pages/SlowQueries'));
const Alerts = lazy(() => import('./pages/Alerts'));
const Security = lazy(() => import('./pages/Security'));
const Encryption = lazy(() => import('./pages/Encryption'));
const DataMasking = lazy(() => import('./pages/DataMasking'));
const AuditLogs = lazy(() => import('./pages/AuditLogs'));
const Backup = lazy(() => import('./pages/Backup'));
const BackupSchedules = lazy(() => import('./pages/BackupSchedules'));
const Restore = lazy(() => import('./pages/Restore'));
const Cluster = lazy(() => import('./pages/Cluster'));
const Replication = lazy(() => import('./pages/Replication'));
const Failover = lazy(() => import('./pages/Failover'));
const Configuration = lazy(() => import('./pages/Configuration'));
const PerformanceConfig = lazy(() => import('./pages/PerformanceConfig'));
const SecurityConfig = lazy(() => import('./pages/SecurityConfig'));
const ResourceGroups = lazy(() => import('./pages/ResourceGroups'));
const ConnectionPools = lazy(() => import('./pages/ConnectionPools'));
const Indexes = lazy(() => import('./pages/Indexes'));
const MaterializedViews = lazy(() => import('./pages/MaterializedViews'));
const StoredProcedures = lazy(() => import('./pages/StoredProcedures'));
const Network = lazy(() => import('./pages/Network'));
const Login = lazy(() => import('./pages/Login'));
const NotFound = lazy(() => import('./pages/NotFound'));

export default function App() {
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) {
    return <LoadingScreen />;
  }

  return (
    <AnimatePresence mode="wait">
      <Suspense fallback={<LoadingScreen />}>
        <Routes>
          {/* Public routes */}
          <Route
            path="/login"
            element={
              isAuthenticated ? <Navigate to="/" replace /> : <Login />
            }
          />

          {/* Protected routes with main layout */}
          <Route
            element={
              <ProtectedRoute>
                <MainLayout />
              </ProtectedRoute>
            }
          >
            {/* Dashboard */}
            <Route index element={<Dashboard />} />
            <Route path="dashboard" element={<Dashboard />} />

            {/* Query Editor */}
            <Route path="query" element={<QueryEditor />} />
            <Route path="query/:queryId" element={<QueryEditor />} />

            {/* Schema Management */}
            <Route path="tables" element={<TableManagement />} />
            <Route path="tables/:tableName" element={<TableDetails />} />
            <Route path="indexes" element={<Indexes />} />
            <Route path="views" element={<MaterializedViews />} />
            <Route path="procedures" element={<StoredProcedures />} />

            {/* User & Role Management */}
            <Route path="users" element={<UserManagement />} />
            <Route path="roles" element={<RoleManagement />} />

            {/* Monitoring */}
            <Route path="monitoring" element={<Monitoring />} />
            <Route path="monitoring/sessions" element={<Sessions />} />
            <Route path="monitoring/slow-queries" element={<SlowQueries />} />
            <Route path="monitoring/alerts" element={<Alerts />} />

            {/* Security */}
            <Route path="security" element={<Security />} />
            <Route path="security/encryption" element={<Encryption />} />
            <Route path="security/masking" element={<DataMasking />} />
            <Route path="security/audit" element={<AuditLogs />} />

            {/* Backup & Recovery */}
            <Route path="backup" element={<Backup />} />
            <Route path="backup/schedules" element={<BackupSchedules />} />
            <Route path="backup/restore" element={<Restore />} />

            {/* Cluster Management */}
            <Route path="cluster" element={<Cluster />} />
            <Route path="cluster/replication" element={<Replication />} />
            <Route path="cluster/failover" element={<Failover />} />

            {/* Network Management */}
            <Route path="network" element={<Network />} />

            {/* Configuration */}
            <Route path="config" element={<Configuration />} />
            <Route path="config/performance" element={<PerformanceConfig />} />
            <Route path="config/security" element={<SecurityConfig />} />

            {/* Resource Management */}
            <Route path="resources" element={<ResourceGroups />} />
            <Route path="resources/pools" element={<ConnectionPools />} />
          </Route>

          {/* 404 */}
          <Route path="*" element={<NotFound />} />
        </Routes>
      </Suspense>
    </AnimatePresence>
  );
}
