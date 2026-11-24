/*
 * Sandbox self-test: tries to open a TCP connection to a public address.
 *
 * Prints CONNECT_FAILED and exits 0 if connect() fails (the expected result
 * inside the sandbox). Prints CONNECT_SUCCEEDED and exits 1 if it succeeds,
 * which means the sandbox leaks network access.
 *
 * Used by crates/otb-sandbox/tests/isolation.rs under both backends.
 */
#include <arpa/inet.h>
#include <errno.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

int main(void) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        printf("CONNECT_FAILED socket errno=%d (%s)\n", errno, strerror(errno));
        return 0;
    }
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof addr);
    addr.sin_family = AF_INET;
    addr.sin_port = htons(443);
    /* 1.1.1.1, a public anycast address. */
    addr.sin_addr.s_addr = htonl(0x01010101u);
    if (connect(fd, (struct sockaddr *)&addr, sizeof addr) != 0) {
        printf("CONNECT_FAILED errno=%d (%s)\n", errno, strerror(errno));
        close(fd);
        return 0;
    }
    printf("CONNECT_SUCCEEDED\n");
    close(fd);
    return 1;
}
