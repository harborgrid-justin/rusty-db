# RustyDB v0.6.0 - Frontend Integration Guide

**Version**: 0.6.0
**Release**: $856M Enterprise Server
**Framework**: React 18 + TypeScript 5
**Build Tool**: Vite
**Last Updated**: 2025-12-28

---

## Table of Contents

1. [Overview](#overview)
2. [Technology Stack](#technology-stack)
3. [Project Structure](#project-structure)
4. [Getting Started](#getting-started)
5. [State Management](#state-management)
6. [API Integration](#api-integration)
7. [Component Development](#component-development)
8. [Real-Time Features](#real-time-features)
9. [Build and Deployment](#build-and-deployment)
10. [Testing](#testing)

---

## Overview

The RustyDB frontend is a modern React-based single-page application (SPA) providing comprehensive database administration, monitoring, and management capabilities.

### Key Features

- **Dashboard**: Real-time metrics and system health monitoring
- **Query Editor**: SQL editor with syntax highlighting and auto-completion
- **Schema Management**: Visual table browser and DDL editor
- **User & Role Management**: Complete RBAC administration
- **Monitoring**: Performance metrics, session tracking, slow query analysis
- **Security**: Encryption, data masking, audit log viewer
- **Backup & Recovery**: Backup scheduling and point-in-time recovery
- **Cluster Management**: Multi-node cluster administration

### Architecture Layers

1. **Presentation Layer**: React components, pages, layouts
2. **State Management Layer**: React Context + Custom hooks
3. **Business Logic Layer**: Service layer with domain logic
4. **Integration Layer**: API clients (REST, WebSocket, GraphQL)
5. **Infrastructure Layer**: Routing, authentication, error handling

---

## Technology Stack

### Core Technologies

| Technology | Version | Purpose |
|------------|---------|---------|
| React | 18.2+ | UI framework |
| TypeScript | 5.0+ | Type safety |
| Vite | 5.0+ | Build tool & dev server |
| React Router | 6.20+ | Client-side routing |

### UI Libraries

| Library | Purpose |
|---------|---------|
| TailwindCSS | Utility-first CSS framework |
| Headless UI | Unstyled accessible components |
| Framer Motion | Animations |
| React Icons | Icon library |
| Recharts | Charts and graphs |
| Monaco Editor | Code editor (SQL) |
| React Query | Server state management |

### Utilities

| Library | Purpose |
|---------|---------|
| Axios | HTTP client |
| date-fns | Date manipulation |
| Zod | Schema validation |
| React Hook Form | Form management |
| React Error Boundary | Error handling |

---

## Project Structure

```
frontend/
├── public/              # Static assets
├── src/
│   ├── components/      # Reusable components
│   │   ├── common/      # Generic components (Button, Input, Card)
│   │   ├── layout/      # Layout components (MainLayout, Sidebar)
│   │   ├── auth/        # Authentication components
│   │   ├── dashboard/   # Dashboard-specific components
│   │   ├── query/       # Query editor components
│   │   ├── schema/      # Schema management components
│   │   ├── monitoring/  # Monitoring components
│   │   └── security/    # Security components
│   ├── contexts/        # React contexts
│   │   ├── AuthContext.tsx
│   │   ├── ThemeContext.tsx
│   │   └── WebSocketContext.tsx
│   ├── hooks/           # Custom React hooks
│   │   ├── useAuth.ts
│   │   ├── useTheme.ts
│   │   └── useWebSocket.ts
│   ├── pages/           # Page components (route targets)
│   │   ├── Dashboard.tsx
│   │   ├── QueryEditor.tsx
│   │   ├── TableManagement.tsx
│   │   └── ...
│   ├── services/        # API service layer
│   │   ├── api.ts       # Base API client
│   │   ├── authService.ts
│   │   ├── queryService.ts
│   │   └── ...
│   ├── types/           # TypeScript type definitions
│   ├── utils/           # Utility functions
│   ├── styles/          # Global styles
│   ├── App.tsx          # Main app component
│   └── main.tsx         # Entry point
├── package.json
├── tsconfig.json
├── tailwind.config.js
└── vite.config.ts
```

---

## Getting Started

### Installation

```bash
cd frontend
npm install
```

### Development Server

```bash
# Start development server (http://localhost:3000)
npm run dev

# Development server with proxy to backend
VITE_API_URL=http://localhost:8080 npm run dev
```

### Build for Production

```bash
# Create production build
npm run build

# Preview production build
npm run preview
```

### Environment Variables

Create `.env.local`:

```bash
VITE_API_URL=http://localhost:8080
VITE_WS_URL=ws://localhost:8080
VITE_GRAPHQL_URL=http://localhost:8080/graphql
```

---

## State Management

### AuthContext

Manages authentication state and user session.

**Usage**:
```typescript
import { useAuth } from '@/hooks/useAuth';

function MyComponent() {
  const { user, isAuthenticated, login, logout } = useAuth();

  if (!isAuthenticated) {
    return <LoginForm onSubmit={login} />;
  }

  return <div>Welcome, {user.displayName}!</div>;
}
```

**Features**:
- Automatic token refresh
- Session persistence (localStorage)
- Permission checking
- Role-based access control

### ThemeContext

Manages application theme (light/dark mode).

**Usage**:
```typescript
import { useTheme } from '@/hooks/useTheme';

function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();

  return (
    <button onClick={toggleTheme}>
      {theme === 'light' ? 'Dark Mode' : 'Light Mode'}
    </button>
  );
}
```

### WebSocketContext

Manages WebSocket connection for real-time updates.

**Usage**:
```typescript
import { useWebSocket } from '@/hooks/useWebSocket';

function RealTimeMetrics() {
  const { subscribe, unsubscribe } = useWebSocket();

  useEffect(() => {
    subscribe('metrics', (data) => {
      setMetrics(data);
    });

    return () => unsubscribe('metrics');
  }, []);

  return <MetricsDisplay metrics={metrics} />;
}
```

---

## API Integration

### Base API Client

**Location**: `src/services/api.ts`

```typescript
import axios from 'axios';

const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_URL || 'http://localhost:8080',
  timeout: 30000,
  headers: {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
  },
});

// Request interceptor - add auth token
apiClient.interceptors.request.use((config) => {
  const auth = localStorage.getItem('rustydb_auth');
  if (auth) {
    const { session } = JSON.parse(auth);
    config.headers.Authorization = `Bearer ${session.token}`;
  }
  return config;
});

// Response interceptor - handle errors
apiClient.interceptors.response.use(
  (response) => response,
  async (error) => {
    // Handle 401 (token expired)
    if (error.response?.status === 401) {
      // Redirect to login
      window.location.href = '/login';
    }
    return Promise.reject(error);
  }
);

export default apiClient;
```

### Service Modules

**Auth Service** (`src/services/authService.ts`):
```typescript
export const authService = {
  async login(username: string, password: string): Promise<AuthResponse> {
    const response = await api.post<ApiResponse<AuthResponse>>('/auth/login', {
      username,
      password,
    });
    return response.data.data;
  },

  async logout(): Promise<void> {
    await api.post('/auth/logout');
  },

  async refreshToken(refreshToken: string): Promise<Session> {
    const response = await api.post<ApiResponse<Session>>('/auth/refresh', {
      refreshToken,
    });
    return response.data.data;
  },
};
```

**Query Service** (`src/services/queryService.ts`):
```typescript
export const queryService = {
  async execute(sql: string, params?: any[]): Promise<QueryResult> {
    const response = await api.post<ApiResponse<QueryResult>>('/query', {
      sql,
      params,
    });
    return response.data.data;
  },

  async explain(sql: string, analyze: boolean = false): Promise<QueryPlan> {
    const response = await api.post<ApiResponse<QueryPlan>>('/query/explain', {
      sql,
      analyze,
    });
    return response.data.data;
  },
};
```

**Monitoring Service** (`src/services/monitoringService.ts`):
```typescript
export const monitoringService = {
  async getMetrics(): Promise<Metrics> {
    const response = await api.get<ApiResponse<Metrics>>('/metrics');
    return response.data.data;
  },

  async getSessionStats(): Promise<SessionStats> {
    const response = await api.get<ApiResponse<SessionStats>>('/stats/sessions');
    return response.data.data;
  },

  async getSlowQueries(limit: number = 100): Promise<SlowQuery[]> {
    const response = await api.get<ApiResponse<SlowQuery[]>>(`/stats/queries?limit=${limit}`);
    return response.data.data.slow_queries;
  },
};
```

---

## Component Development

### Common Components

**Button Component** (`src/components/common/Button.tsx`):
```typescript
interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  loading?: boolean;
  children: React.ReactNode;
  onClick?: () => void;
}

export function Button({
  variant = 'primary',
  size = 'md',
  disabled = false,
  loading = false,
  children,
  onClick,
}: ButtonProps) {
  const baseClasses = 'rounded font-medium transition-colors';
  const variantClasses = {
    primary: 'bg-blue-600 text-white hover:bg-blue-700',
    secondary: 'bg-gray-200 text-gray-900 hover:bg-gray-300',
    danger: 'bg-red-600 text-white hover:bg-red-700',
    ghost: 'bg-transparent text-gray-700 hover:bg-gray-100',
  };
  const sizeClasses = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-6 py-3 text-lg',
  };

  return (
    <button
      className={cn(baseClasses, variantClasses[variant], sizeClasses[size])}
      disabled={disabled || loading}
      onClick={onClick}
    >
      {loading && <Spinner className="mr-2" />}
      {children}
    </button>
  );
}
```

**Table Component** (`src/components/common/Table.tsx`):
```typescript
interface Column<T> {
  key: keyof T;
  header: string;
  render?: (value: any, row: T) => React.ReactNode;
}

interface TableProps<T> {
  columns: Column<T>[];
  data: T[];
  loading?: boolean;
  onRowClick?: (row: T) => void;
}

export function Table<T extends Record<string, any>>({
  columns,
  data,
  loading,
  onRowClick,
}: TableProps<T>) {
  if (loading) {
    return <LoadingSpinner />;
  }

  return (
    <div className="overflow-x-auto">
      <table className="min-w-full divide-y divide-gray-200">
        <thead className="bg-gray-50 dark:bg-gray-700">
          <tr>
            {columns.map((column) => (
              <th
                key={String(column.key)}
                className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase"
              >
                {column.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="bg-white divide-y divide-gray-200">
          {data.map((row, index) => (
            <tr
              key={index}
              onClick={() => onRowClick?.(row)}
              className="hover:bg-gray-50 cursor-pointer"
            >
              {columns.map((column) => (
                <td key={String(column.key)} className="px-6 py-4">
                  {column.render
                    ? column.render(row[column.key], row)
                    : row[column.key]}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

### Protected Routes

```typescript
import { Navigate } from 'react-router-dom';
import { useAuth } from '@/hooks/useAuth';

export function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) {
    return <LoadingScreen />;
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return <>{children}</>;
}
```

### Permission-Based Rendering

```typescript
export function PermissionGate({
  resource,
  action,
  children,
  fallback = null,
}: {
  resource: string;
  action: string;
  children: React.ReactNode;
  fallback?: React.ReactNode;
}) {
  const { hasPermission } = useAuth();

  if (!hasPermission(resource, action)) {
    return <>{fallback}</>;
  }

  return <>{children}</>;
}
```

---

## Real-Time Features

### WebSocket Integration

**Connection Management** (`src/contexts/WebSocketContext.tsx`):
```typescript
export function WebSocketProvider({ children }: { children: React.ReactNode }) {
  const [ws, setWs] = useState<WebSocket | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    const wsUrl = `${import.meta.env.VITE_WS_URL}/stream`;
    const socket = new WebSocket(wsUrl);

    socket.onopen = () => {
      console.log('WebSocket connected');
      setIsConnected(true);
    };

    socket.onmessage = (event) => {
      const message = JSON.parse(event.data);
      // Handle message...
    };

    socket.onclose = () => {
      console.log('WebSocket disconnected');
      setIsConnected(false);
      // Reconnect after 5 seconds
      setTimeout(() => connect(), 5000);
    };

    setWs(socket);

    return () => socket.close();
  }, []);

  return (
    <WebSocketContext.Provider value={{ isConnected, ws }}>
      {children}
    </WebSocketContext.Provider>
  );
}
```

### Real-Time Metrics Component

```typescript
export function RealTimeMetrics() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const { subscribe, unsubscribe } = useWebSocket();

  useEffect(() => {
    const handleMetrics = (data: Metrics) => {
      setMetrics(data);
    };

    subscribe('metrics', handleMetrics);

    return () => unsubscribe('metrics', handleMetrics);
  }, [subscribe, unsubscribe]);

  if (!metrics) {
    return <LoadingSpinner />;
  }

  return (
    <div className="grid grid-cols-4 gap-4">
      <MetricCard
        title="Queries/sec"
        value={metrics.queries_per_second}
        trend="up"
      />
      <MetricCard
        title="Active Connections"
        value={metrics.active_connections}
      />
      {/* More metrics... */}
    </div>
  );
}
```

---

## Build and Deployment

### Vite Configuration

**`vite.config.ts`**:
```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom', 'react-router-dom'],
          ui: ['@headlessui/react', 'framer-motion'],
          charts: ['recharts'],
          editor: ['monaco-editor'],
        },
      },
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:8080',
        ws: true,
      },
    },
  },
});
```

### Docker Deployment

**Dockerfile**:
```dockerfile
# Build stage
FROM node:18-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Production stage
FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

**nginx.conf**:
```nginx
server {
  listen 80;
  root /usr/share/nginx/html;
  index index.html;

  location / {
    try_files $uri $uri/ /index.html;
  }

  location /api {
    proxy_pass http://rustydb-api:8080;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection 'upgrade';
    proxy_set_header Host $host;
  }

  location /ws {
    proxy_pass http://rustydb-api:8080;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "Upgrade";
  }
}
```

---

## Testing

### Unit Testing (Vitest + React Testing Library)

```typescript
import { render, screen, fireEvent } from '@testing-library/react';
import { Button } from '@/components/common/Button';

describe('Button', () => {
  it('renders with children', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByText('Click me')).toBeInTheDocument();
  });

  it('calls onClick when clicked', () => {
    const handleClick = vi.fn();
    render(<Button onClick={handleClick}>Click me</Button>);
    fireEvent.click(screen.getByText('Click me'));
    expect(handleClick).toHaveBeenCalledOnce();
  });

  it('shows loading state', () => {
    render(<Button loading>Click me</Button>);
    expect(screen.getByRole('button')).toBeDisabled();
  });
});
```

### E2E Testing (Playwright)

```typescript
import { test, expect } from '@playwright/test';

test('user can login and view dashboard', async ({ page }) => {
  await page.goto('http://localhost:3000');

  // Login
  await page.fill('input[name="username"]', 'admin');
  await page.fill('input[name="password"]', 'password');
  await page.click('button[type="submit"]');

  // Wait for dashboard
  await expect(page).toHaveURL(/.*dashboard/);
  await expect(page.locator('h1')).toContainText('Dashboard');

  // Check metrics are loaded
  await expect(page.locator('.metric-card')).toHaveCount(4);
});
```

---

## Performance Optimization

### Code Splitting

```typescript
import { lazy, Suspense } from 'react';

const Dashboard = lazy(() => import('./pages/Dashboard'));
const QueryEditor = lazy(() => import('./pages/QueryEditor'));

function App() {
  return (
    <Suspense fallback={<LoadingScreen />}>
      <Routes>
        <Route path="/" element={<Dashboard />} />
        <Route path="/query" element={<QueryEditor />} />
      </Routes>
    </Suspense>
  );
}
```

### Memoization

```typescript
import { memo, useMemo } from 'react';

// Component memoization
export const TableRow = memo(function TableRow({ row, onRowClick }) {
  return (
    <tr onClick={() => onRowClick(row)}>
      {/* ... */}
    </tr>
  );
});

// Value memoization
function DataTable({ data, filter }) {
  const filteredData = useMemo(() => {
    return data.filter((item) => item.name.includes(filter));
  }, [data, filter]);

  return <Table data={filteredData} />;
}
```

---

## Resources

- **React Documentation**: https://react.dev
- **TypeScript Documentation**: https://www.typescriptlang.org/docs/
- **Vite Documentation**: https://vitejs.dev
- **TailwindCSS**: https://tailwindcss.com/docs

---

**For backend integration details, see [NODEJS_ADAPTER.md](./NODEJS_ADAPTER.md) and the API documentation in `release/docs/0.6/api/`.**
