# RustyDB Enterprise Management Platform

A production-ready, enterprise-grade web-based management platform for RustyDB - the high-performance database system written in Rust.

![RustyDB Logo](./public/favicon.svg)

## 🚀 Overview

The RustyDB Management Platform provides a comprehensive web interface for managing all aspects of your RustyDB database infrastructure. Built with modern technologies and enterprise-grade security, it offers:

- **Real-time Monitoring** - Live dashboards with metrics, alerts, and performance insights
- **SQL Query Editor** - Feature-rich editor with syntax highlighting, autocomplete, and execution plans
- **Schema Management** - Tables, indexes, views, and stored procedures management
- **Security Center** - Encryption, data masking, audit logs, and compliance tools
- **Backup & Recovery** - Automated backups, scheduling, and point-in-time recovery
- **Cluster Management** - Node topology, replication monitoring, and failover control
- **Resource Management** - Connection pools, resource groups, and workload management

## 📋 Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Configuration](#configuration)
- [Development](#development)
- [Production Deployment](#production-deployment)
- [Architecture](#architecture)
- [Features](#features)
- [API Integration](#api-integration)
- [Security](#security)
- [Troubleshooting](#troubleshooting)

## 🔧 Prerequisites

Before you begin, ensure you have the following installed:

| Tool | Version | Purpose |
|------|---------|---------|
| Node.js | >= 18.0.0 | JavaScript runtime |
| npm | >= 9.0.0 | Package manager |
| Git | >= 2.0 | Version control |
| Docker (optional) | >= 20.0 | Containerized deployment |

## ⚡ Quick Start

```bash
# Clone the repository (if not already done)
cd /path/to/rusty-db/frontend

# Install dependencies
npm install

# Copy environment configuration
cp .env.example .env

# Start development server
npm run dev

# Open in browser
open http://localhost:3000
```

## 📦 Installation

### Step 1: Install Dependencies

```bash
# Install all project dependencies
npm install

# Verify installation
npm ls --depth=0
```

### Step 2: Environment Configuration

```bash
# Copy the example environment file
cp .env.example .env

# Edit the configuration
nano .env  # or your preferred editor
```

**Essential environment variables:**

```env
# API Configuration
VITE_API_URL=http://localhost:8080          # RustyDB REST API endpoint
VITE_GRAPHQL_URL=http://localhost:8080/graphql  # GraphQL endpoint
VITE_WS_URL=ws://localhost:8080/ws          # WebSocket endpoint

# Authentication
VITE_AUTH_ENABLED=true                       # Enable authentication (production)

# Features
VITE_ENABLE_CLUSTER_MANAGEMENT=true          # Cluster features
VITE_ENABLE_REALTIME_MONITORING=true         # Real-time updates
```

### Step 3: Start Development Server

```bash
npm run dev
```

The application will be available at `http://localhost:3000`

## ⚙️ Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_API_URL` | `http://localhost:8080` | RustyDB REST API base URL |
| `VITE_GRAPHQL_URL` | `http://localhost:8080/graphql` | GraphQL endpoint |
| `VITE_WS_URL` | `ws://localhost:8080/ws` | WebSocket for real-time updates |
| `VITE_AUTH_ENABLED` | `false` | Enable authentication |
| `VITE_SESSION_TIMEOUT` | `1800000` | Session timeout (ms) |
| `VITE_METRICS_REFRESH_INTERVAL` | `5000` | Metrics refresh rate (ms) |
| `VITE_DEFAULT_THEME` | `dark` | Default UI theme |
| `VITE_DEFAULT_PAGE_SIZE` | `50` | Table pagination size |
| `VITE_ENABLE_DEVTOOLS` | `true` | React Query devtools |

### Theme Configuration

The platform supports light and dark themes with automatic system detection:

```typescript
// In your component
import { useTheme } from '@/contexts/ThemeContext';

function MyComponent() {
  const { theme, setTheme, toggleTheme } = useTheme();
  // theme: 'light' | 'dark' | 'system'
}
```

## 💻 Development

### Available Scripts

```bash
# Development
npm run dev              # Start dev server with hot reload
npm run build            # Build for production
npm run preview          # Preview production build

# Code Quality
npm run lint             # Run ESLint
npm run lint:fix         # Fix ESLint errors
npm run format           # Format with Prettier
npm run type-check       # TypeScript type checking

# Testing
npm run test             # Run tests
npm run test:coverage    # Run tests with coverage
npm run test:ui          # Open Vitest UI

# Analysis
npm run analyze          # Bundle size analysis
```

### Project Structure

```
frontend/
├── public/                  # Static assets
├── src/
│   ├── components/          # React components
│   │   ├── auth/           # Authentication components
│   │   ├── backup/         # Backup management
│   │   ├── cluster/        # Cluster management
│   │   ├── common/         # Shared UI components
│   │   ├── config/         # Configuration components
│   │   ├── dashboard/      # Dashboard widgets
│   │   ├── layout/         # Layout components
│   │   ├── monitoring/     # Monitoring components
│   │   ├── query/          # Query editor
│   │   ├── resources/      # Resource management
│   │   ├── roles/          # Role management
│   │   ├── schema/         # Schema management
│   │   ├── security/       # Security components
│   │   └── users/          # User management
│   ├── contexts/            # React contexts
│   ├── hooks/               # Custom React hooks
│   ├── pages/               # Page components
│   ├── services/            # API services
│   ├── stores/              # Zustand stores
│   ├── styles/              # Global styles
│   ├── types/               # TypeScript types
│   ├── utils/               # Utility functions
│   ├── App.tsx              # Main App component
│   └── main.tsx             # Entry point
├── .env.example             # Environment template
├── index.html               # HTML template
├── package.json             # Dependencies
├── tailwind.config.js       # Tailwind CSS config
├── tsconfig.json            # TypeScript config
└── vite.config.ts           # Vite config
```

### Component Development

All components follow a consistent pattern:

```typescript
// Example: components/common/Button.tsx
import { type ButtonHTMLAttributes, forwardRef } from 'react';
import clsx from 'clsx';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  isLoading?: boolean;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = 'primary', size = 'md', isLoading, children, ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={clsx('btn', `btn-${variant}`, `btn-${size}`)}
        disabled={isLoading}
        {...props}
      >
        {isLoading ? <LoadingSpinner /> : children}
      </button>
    );
  }
);
```

### Adding New Features

1. **Create the service** in `src/services/`
2. **Create hooks** in `src/hooks/`
3. **Create components** in `src/components/`
4. **Create page** in `src/pages/`
5. **Add route** in `src/App.tsx`
6. **Add navigation** in `src/components/layout/MainLayout.tsx`

## 🚢 Production Deployment

### Option 1: Docker Deployment (Recommended)

```bash
# Build Docker image
docker build -t rustydb-frontend .

# Run container
docker run -d \
  --name rustydb-frontend \
  -p 3000:80 \
  -e VITE_API_URL=http://your-api-server:8080 \
  rustydb-frontend
```

### Option 2: Docker Compose

```bash
# Start with docker-compose
docker-compose up -d

# View logs
docker-compose logs -f frontend

# Stop
docker-compose down
```

### Option 3: Manual Build

```bash
# Build production bundle
npm run build:prod

# The output is in ./dist directory
# Serve with any static file server

# Example with nginx
sudo cp -r dist/* /var/www/rustydb/

# Example with serve
npx serve -s dist -l 3000
```

### Nginx Configuration

```nginx
server {
    listen 80;
    server_name rustydb.example.com;
    root /var/www/rustydb;
    index index.html;

    # Gzip compression
    gzip on;
    gzip_types text/plain text/css application/json application/javascript;

    # Static assets caching
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # SPA fallback
    location / {
        try_files $uri $uri/ /index.html;
    }

    # API proxy
    location /api/ {
        proxy_pass http://rustydb-api:8080/api/;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }

    # WebSocket proxy
    location /ws {
        proxy_pass http://rustydb-api:8080/ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header Host $host;
    }

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
}
```

## 🏗️ Architecture

### Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| UI Framework | React 18 | Component-based UI |
| Language | TypeScript | Type safety |
| Styling | Tailwind CSS | Utility-first CSS |
| State Management | Zustand + React Query | Client and server state |
| Routing | React Router v6 | SPA navigation |
| HTTP Client | Axios | API communication |
| Charts | Recharts | Data visualization |
| Editor | Monaco Editor | SQL editing |
| Animations | Framer Motion | UI animations |
| Icons | Heroicons | Consistent iconography |

### Data Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   UI Layer  │────▶│  Hook Layer │────▶│   Service   │
│ (Components)│     │(React Query)│     │   Layer     │
└─────────────┘     └─────────────┘     └─────────────┘
                           │                    │
                           │                    │
                    ┌──────▼──────┐      ┌──────▼──────┐
                    │   Zustand   │      │  REST API   │
                    │   Stores    │      │  (RustyDB)  │
                    └─────────────┘      └─────────────┘
```

### State Management Strategy

- **Server State**: React Query for API data with caching, refetching, and invalidation
- **Client State**: Zustand for UI state (sidebar, tabs, preferences)
- **Form State**: React Hook Form with Zod validation
- **URL State**: React Router for navigation state

## ✨ Features

### Dashboard
- Real-time system metrics (CPU, memory, disk, network)
- Query performance charts
- Active session monitoring
- Alert summary
- Health status indicators

### Query Editor
- Monaco Editor with SQL syntax highlighting
- IntelliSense autocomplete for tables and columns
- Query execution with results grid
- Explain plan visualization
- Query history and saved queries
- Export to CSV/JSON

### Schema Management
- Table browser with statistics
- Column editor with all data types
- Index management with usage stats
- Foreign key visualization
- Materialized view management
- Stored procedure editor

### User Management
- User CRUD operations
- Role-based access control
- Permission matrix editor
- Session management
- Password policies

### Monitoring
- Active session viewer
- Slow query analysis
- Blocking tree visualization
- Performance gauges
- Alert management

### Security
- Encryption key management
- Key rotation wizard
- Data masking policies
- Audit log viewer
- Security event timeline

### Backup & Recovery
- Backup list with status
- Scheduled backups with cron
- Point-in-time recovery
- Restore wizard
- Storage usage tracking

### Cluster Management
- Interactive topology diagram
- Node health monitoring
- Replication lag charts
- Failover wizard
- Add/remove nodes

### Configuration
- Memory configuration sliders
- Performance tuning
- Security settings
- Configuration history
- Import/export settings

## 🔌 API Integration

### REST API Endpoints

The frontend expects these API endpoints from RustyDB:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/auth/login` | POST | User authentication |
| `/api/v1/query` | POST | Execute SQL query |
| `/api/v1/tables` | GET | List all tables |
| `/api/v1/metrics` | GET | System metrics |
| `/api/v1/admin/users` | GET/POST | User management |
| `/api/v1/admin/backup` | POST | Create backup |
| `/api/v1/cluster/nodes` | GET | Cluster topology |

See the full API documentation in the RustyDB main repository.

### WebSocket Events

Real-time updates are received via WebSocket:

```typescript
// Event types
type WebSocketEvent =
  | 'metrics.update'
  | 'alert.new'
  | 'session.change'
  | 'replication.lag'
  | 'cluster.topology';
```

## 🔐 Security

### Authentication

The platform supports multiple authentication methods:

1. **Username/Password** - Traditional login
2. **Session Tokens** - JWT-based sessions
3. **Development Mode** - Auth disabled for local development

### Security Headers

The application sets these security headers (via nginx or CDN):

- `X-Frame-Options: DENY`
- `X-Content-Type-Options: nosniff`
- `X-XSS-Protection: 1; mode=block`
- `Content-Security-Policy: ...`

### Best Practices

- All API calls use HTTPS in production
- Tokens stored in httpOnly cookies (when available)
- Automatic session timeout
- CSRF protection via tokens
- Input sanitization on all forms

## 🐛 Troubleshooting

### Common Issues

**1. Connection refused to API**
```bash
# Check if RustyDB is running
curl http://localhost:8080/api/v1/health

# Update .env if using different port
VITE_API_URL=http://localhost:YOUR_PORT
```

**2. WebSocket not connecting**
```bash
# Ensure WebSocket endpoint is correct
VITE_WS_URL=ws://localhost:8080/ws

# Check browser console for errors
```

**3. Build fails with memory error**
```bash
# Increase Node.js memory
export NODE_OPTIONS="--max-old-space-size=4096"
npm run build
```

**4. TypeScript errors**
```bash
# Clear cache and reinstall
rm -rf node_modules/.cache
npm run type-check
```

### Debug Mode

Enable debug logging:

```env
VITE_LOG_LEVEL=debug
VITE_DEV_MODE=true
```

### Getting Help

- Check the [Issues](https://github.com/your-repo/rusty-db/issues) page
- Review the API documentation
- Check browser developer console for errors

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

- Built with [React](https://react.dev/)
- Styled with [Tailwind CSS](https://tailwindcss.com/)
- Charts by [Recharts](https://recharts.org/)
- Icons from [Heroicons](https://heroicons.com/)
- Editor powered by [Monaco Editor](https://microsoft.github.io/monaco-editor/)

---

**RustyDB Management Platform** - Enterprise-grade database management made simple.
