#include "backend.h"

#include <QJsonDocument>
#include <QJsonObject>

Backend::Backend(const QUrl &url, bool connectNow, QObject *parent) : QObject(parent), m_url(url) {
    // voiced may start after the face, or restart; keep trying every two seconds.
    m_retry.setInterval(2000);
    m_retry.setSingleShot(true);
    connect(&m_retry, &QTimer::timeout, this, [this] { m_socket.open(m_url); });
    connect(&m_socket, &QWebSocket::connected, this, [this] { setConnected(true); });
    connect(&m_socket, &QWebSocket::disconnected, this, [this] {
        setConnected(false);
        if (!m_demo)
            setState("idle");
        m_retry.start();
    });
    connect(&m_socket, &QWebSocket::textMessageReceived, this, &Backend::handleMessage);
    if (connectNow)
        m_socket.open(m_url);
}

void Backend::handleMessage(const QString &message) {
    const QJsonObject object = QJsonDocument::fromJson(message.toUtf8()).object();
    const QString state = object.value("state").toString();
    if (state.isEmpty())
        return;
    if (m_demo) {
        m_demo = false;
        emit demoChanged();
    }

    const QString text = object.value("text").toString();
    if (state == "heard") {
        // What Luna heard arrives while she is already thinking about it.
        setHeard(text);
        setState("thinking");
        return;
    }
    if (state == "listening" || state == "wake")
        setSpeech({});
    if (state == "wake" || state == "idle")
        setHeard({});
    if (state == "speaking")
        setSpeech(text);
    setTool(state == "thinking" ? object.value("tool").toString() : QString());
    setState(state);
}

void Backend::stepDemo() {
    struct Step { const char *state, *speech, *heard, *tool; };
    static const Step steps[] = {
        {"wake", "", "", ""},
        {"listening", "", "", ""},
        {"thinking", "", "Pull up a pancake recipe.", ""},
        {"thinking", "", "Pull up a pancake recipe.", "web_search"},
        {"speaking", "Fluffy pancakes are up on the screen. About twenty minutes start to finish.",
         "Pull up a pancake recipe.", ""},
        {"idle", "", "", ""},
    };
    m_demoStep = (m_demoStep + 1) % int(std::size(steps));
    if (!m_demo) {
        m_demo = true;
        emit demoChanged();
    }
    const Step &step = steps[m_demoStep];
    setSpeech(QString::fromUtf8(step.speech));
    setHeard(QString::fromUtf8(step.heard));
    setTool(QString::fromUtf8(step.tool));
    setState(QString::fromUtf8(step.state));
}

void Backend::setState(const QString &state) {
    if (state != m_state) {
        m_state = state;
        emit stateChanged();
    }
}

void Backend::setSpeech(const QString &speech) {
    if (speech != m_speech) {
        m_speech = speech;
        emit speechChanged();
    }
}

void Backend::setHeard(const QString &heard) {
    if (heard != m_heard) {
        m_heard = heard;
        emit heardChanged();
    }
}

void Backend::setTool(const QString &tool) {
    if (tool != m_tool) {
        m_tool = tool;
        emit toolChanged();
    }
}

void Backend::setConnected(bool connected) {
    if (connected != m_connected) {
        m_connected = connected;
        emit connectedChanged();
    }
}
