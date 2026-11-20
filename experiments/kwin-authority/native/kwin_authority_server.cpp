// SPDX-License-Identifier: GPL-2.0-or-later
// Private Native Authority IPC Server for Phase 3.10.1 cross-process integration.

#include "authority.h"
#include <QCoreApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <iostream>
#include <memory>
#include <string>
#include <vector>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>
#include <fcntl.h>
#include <poll.h>

using namespace depthos::experiment;
using namespace depthos::experiment::synthetic;

static void setNonBlocking(int fd) {
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags >= 0) {
        fcntl(fd, F_SETFL, flags | O_NONBLOCK);
    }
}

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);

    std::string socketPath;
    std::string bootstrapToken = "launcher-bootstrap-token-phase-3-10-1";
    std::string fixtureToken = "fixture-authority-token-kwin-private";
    std::string compositorInc = "10000000-0000-0000-0000-000000000001";
    std::string targetWindowUuid = "00000000-0000-0000-0000-000000000001";
    uint64_t targetGeneration = 1;

    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg == "--socket" && i + 1 < argc) socketPath = argv[++i];
        else if (arg == "--bootstrap-token" && i + 1 < argc) bootstrapToken = argv[++i];
        else if (arg == "--fixture-token" && i + 1 < argc) fixtureToken = argv[++i];
        else if (arg == "--compositor-incarnation" && i + 1 < argc) compositorInc = argv[++i];
        else if (arg == "--window-uuid" && i + 1 < argc) targetWindowUuid = argv[++i];
        else if (arg == "--generation" && i + 1 < argc) targetGeneration = std::stoull(argv[++i]);
    }

    if (socketPath.empty()) {
        std::cerr << "Usage: kwin_authority_server --socket <path> [options]" << std::endl;
        return 1;
    }

    // World & Kernel setup
    std::unique_ptr<QObject> windowObj = std::make_unique<QObject>();
    QObject promptObj;
    World world;
    world.windows.push_back({windowObj.get(), targetWindowUuid, targetGeneration, "source", "act-1", true});
    world.focus = windowObj.get();
    world.prompt = &promptObj;
    world.promptOwned = true;
    world.promptRendered = true;
    world.promptVisible = true;
    world.desktops = {"source", "destination", "extra"};
    world.currentDesktop = "source";
    world.activity = "act-1";
    world.seat = "seat0";
    world.activeSeat = "seat0";
    world.deviceSeat = "seat0";
    world.compositor = 1001;
    world.coordinator = 2001;
    world.observer = 3001;
    world.topology = 1;
    world.lockPhase = LockPhase::Unlocked;

    Kernel kernel;
    SyntheticAuthorityServer server(kernel, world, bootstrapToken, fixtureToken, compositorInc);

    // Unlink old socket if present
    unlink(socketPath.c_str());

    int listenFd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (listenFd < 0) {
        perror("socket");
        return 1;
    }

    sockaddr_un addr{};
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, socketPath.c_str(), sizeof(addr.sun_path) - 1);

    if (bind(listenFd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) < 0) {
        perror("bind");
        close(listenFd);
        return 1;
    }

    // Enforce 0600 on socket
    if (chmod(socketPath.c_str(), 0600) < 0) {
        perror("chmod socket");
        close(listenFd);
        return 1;
    }

    if (listen(listenFd, 5) < 0) {
        perror("listen");
        close(listenFd);
        return 1;
    }

    // Signal readiness
    std::cout << "READY socket=" << socketPath << " comp_inc=" << compositorInc << std::endl;
    std::cout.flush();

    // Event loop handling IPC connections and stdin control commands
    int clientFd = -1;
    bool running = true;

    while (running) {
        pollfd pfd[2];
        pfd[0].fd = (clientFd >= 0) ? clientFd : listenFd;
        pfd[0].events = POLLIN;
        pfd[1].fd = STDIN_FILENO;
        pfd[1].events = POLLIN;

        int ret = poll(pfd, 2, 5000); // 5 sec timeout
        if (ret < 0) {
            if (errno == EINTR) continue;
            break;
        }

        // Handle control input on stdin
        if (pfd[1].revents & POLLIN) {
            std::string line;
            if (std::getline(std::cin, line)) {
                if (line == "focus_loss") {
                    world.focus = nullptr;
                    std::cout << "CONTROL: focus_loss" << std::endl;
                } else if (line == "destroy_window") {
                    world.windows.clear();
                    world.focus = nullptr;
                    std::cout << "CONTROL: destroy_window" << std::endl;
                } else if (line.rfind("recreate_window ", 0) == 0) {
                    uint64_t gen = std::stoull(line.substr(16));
                    world.windows.clear();
                    world.windows.push_back({windowObj.get(), targetWindowUuid, gen, "desktop-1", "act-1", true});
                    world.focus = windowObj.get();
                    std::cout << "CONTROL: recreate_window gen=" << gen << std::endl;
                } else if (line == "remove_desktop") {
                    world.desktops = {"desktop-1"}; // destination "desktop-2" removed
                    std::cout << "CONTROL: remove_desktop" << std::endl;
                } else if (line == "lock") {
                    world.lockPhase = LockPhase::Locked;
                    std::cout << "CONTROL: lock" << std::endl;
                } else if (line == "quit") {
                    running = false;
                    break;
                }
                std::cout.flush();
            } else {
                // Stdin EOF
                running = false;
                break;
            }
        }

        // Handle network connection or message
        if (pfd[0].revents & POLLIN) {
            if (clientFd < 0) {
                // Accept connection
                ucred cred{};
                socklen_t len = sizeof(cred);
                int fd = accept(listenFd, nullptr, nullptr);
                if (fd >= 0) {
                    if (getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &cred, &len) == 0) {
                        if (cred.uid == geteuid() && cred.pid > 0) {
                            clientFd = fd;
                            std::cout << "CLIENT_CONNECTED pid=" << cred.pid << " uid=" << cred.uid << std::endl;
                        } else {
                            std::cerr << "REJECT_PEER_CRED uid=" << cred.uid << " pid=" << cred.pid << std::endl;
                            close(fd);
                        }
                    } else {
                        close(fd);
                    }
                }
            } else {
                // Process frame from client
                bool ok = server.processMessage(clientFd, Kernel::Clock::now());
                if (!ok) {
                    std::cout << "PROCESS_MESSAGE_FAILED or DISCONNECT" << std::endl;
                    close(clientFd);
                    clientFd = -1;
                } else {
                    std::cout << "MESSAGE_PROCESSED count=" << server.count() << std::endl;
                }
                std::cout.flush();
            }
        }

        if (pfd[0].revents & (POLLERR | POLLHUP | POLLNVAL)) {
            if (clientFd >= 0) {
                close(clientFd);
                clientFd = -1;
            }
        }
    }

    if (clientFd >= 0) close(clientFd);
    close(listenFd);
    unlink(socketPath.c_str());
    return 0;
}
