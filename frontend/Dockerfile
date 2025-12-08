# ============================================================================
# RustyDB Frontend - Production Dockerfile
# Multi-stage build for optimized production image
# ============================================================================

# Stage 1: Dependencies
FROM node:20-alpine AS deps
WORKDIR /app

# Install dependencies needed for native modules
RUN apk add --no-cache libc6-compat

# Copy package files
COPY package.json package-lock.json* ./

# Install dependencies
RUN npm ci --legacy-peer-deps

# Stage 2: Build
FROM node:20-alpine AS builder
WORKDIR /app

# Copy dependencies from deps stage
COPY --from=deps /app/node_modules ./node_modules

# Copy source files
COPY . .

# Set build-time environment variables
ARG VITE_API_URL=http://localhost:8080
ARG VITE_GRAPHQL_URL=http://localhost:8080/graphql
ARG VITE_WS_URL=ws://localhost:8080/ws
ARG VITE_AUTH_ENABLED=true
ARG VITE_ENABLE_CLUSTER_MANAGEMENT=true
ARG VITE_ENABLE_REALTIME_MONITORING=true

ENV VITE_API_URL=$VITE_API_URL
ENV VITE_GRAPHQL_URL=$VITE_GRAPHQL_URL
ENV VITE_WS_URL=$VITE_WS_URL
ENV VITE_AUTH_ENABLED=$VITE_AUTH_ENABLED
ENV VITE_ENABLE_CLUSTER_MANAGEMENT=$VITE_ENABLE_CLUSTER_MANAGEMENT
ENV VITE_ENABLE_REALTIME_MONITORING=$VITE_ENABLE_REALTIME_MONITORING

# Build the application
RUN npm run build

# Stage 3: Production
FROM nginx:alpine AS runner
WORKDIR /usr/share/nginx/html

# Install curl for healthcheck
RUN apk add --no-cache curl

# Remove default nginx static files
RUN rm -rf ./*

# Copy built assets from builder stage
COPY --from=builder /app/dist .

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Create non-root user for security
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nextjs -u 1001 && \
    chown -R nextjs:nodejs /usr/share/nginx/html && \
    chown -R nextjs:nodejs /var/cache/nginx && \
    chown -R nextjs:nodejs /var/log/nginx && \
    touch /var/run/nginx.pid && \
    chown -R nextjs:nodejs /var/run/nginx.pid

# Expose port
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost/health || exit 1

# Start nginx
CMD ["nginx", "-g", "daemon off;"]
