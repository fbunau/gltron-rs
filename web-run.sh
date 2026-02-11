#!/usr/bin/env bash
set -e

PORT=8080

build() {
    echo "Building WASM..."
    cargo build --target wasm32-unknown-unknown --release
    wasm-bindgen target/wasm32-unknown-unknown/release/gltron.wasm \
        --out-dir web/pkg --target web --no-typescript
    echo "Build complete. Refresh http://localhost:$PORT to see changes."
}

start_server() {
    echo "Starting server on http://localhost:$PORT ..."
    miniserve web/ --port $PORT --index index.html &
    echo $! > .miniserve.pid
}

stop_server() {
    if [ -f .miniserve.pid ]; then
        pid=$(cat .miniserve.pid)
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            echo "Stopped server (pid $pid)."
        fi
        rm -f .miniserve.pid
    fi
}

server_running() {
    if [ -f .miniserve.pid ]; then
        pid=$(cat .miniserve.pid)
        if kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
    fi
    return 1
}

case "${1:-run}" in
    restart)
        stop_server
        build
        start_server
        ;;
    run|"")
        build
        if ! server_running; then
            start_server
        fi
        ;;
    build)
        build
        ;;
    stop)
        stop_server
        ;;
    *)
        echo "Usage: $0 [run|build|restart|stop]"
        echo "  (no args)  Build and start server if not running"
        echo "  build      Build only"
        echo "  restart    Rebuild and restart server"
        echo "  stop       Stop server"
        exit 1
        ;;
esac
