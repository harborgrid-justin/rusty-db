/*
 * Port Override Shim for RustyDB Multi-Node Testing
 *
 * This LD_PRELOAD library intercepts bind() calls and modifies the port
 * based on environment variables, allowing multiple server instances
 * to run on different ports without code modification.
 *
 * Usage:
 *   RUSTYDB_PORT=5433 RUSTYDB_API_PORT=8081 \
 *   LD_PRELOAD=./port_override.so ./rusty-db-server
 *
 * Compile:
 *   gcc -shared -fPIC -o port_override.so port_override.c -ldl
 */

#define _GNU_SOURCE
#include <dlfcn.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

static int (*real_bind)(int, const struct sockaddr*, socklen_t) = NULL;

int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen) {
    if (!real_bind) {
        real_bind = dlsym(RTLD_NEXT, "bind");
    }

    if (addr->sa_family == AF_INET) {
        struct sockaddr_in *addr_in = (struct sockaddr_in *)addr;
        int orig_port = ntohs(addr_in->sin_port);

        // Check for port override environment variables
        char *port_override = NULL;

        if (orig_port == 5432) {
            port_override = getenv("RUSTYDB_PORT");
        } else if (orig_port == 8080) {
            port_override = getenv("RUSTYDB_API_PORT");
        }

        if (port_override) {
            int new_port = atoi(port_override);
            if (new_port > 0 && new_port < 65536) {
                // Create a modified copy of the address
                struct sockaddr_in modified_addr;
                memcpy(&modified_addr, addr_in, sizeof(struct sockaddr_in));
                modified_addr.sin_port = htons(new_port);

                fprintf(stderr, "[port_override] Redirecting port %d -> %d\n",
                        orig_port, new_port);

                return real_bind(sockfd, (struct sockaddr*)&modified_addr, addrlen);
            }
        }
    }

    return real_bind(sockfd, addr, addrlen);
}
