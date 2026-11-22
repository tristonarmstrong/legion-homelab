#pragma once

#include <QObject>
#include <QTimer>
#include <QUrl>
#include <QWebSocket>

// Follows voiced, Luna's voice loop, over its local WebSocket. voiced sends one
// JSON object per change: {"state": "idle|wake|listening|thinking|heard|speaking",
// "text": ..., "tool": ...}. The face only ever shows what voiced reports, so if
// voiced is not running the window says so instead of pretending to listen.
class Backend : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString state READ state NOTIFY stateChanged)
    Q_PROPERTY(QString speech READ speech NOTIFY speechChanged)
    Q_PROPERTY(QString heard READ heard NOTIFY heardChanged)
    Q_PROPERTY(QString tool READ tool NOTIFY toolChanged)
    Q_PROPERTY(bool connected READ connected NOTIFY connectedChanged)
    Q_PROPERTY(bool demo READ demo NOTIFY demoChanged)

public:
    explicit Backend(const QUrl &url = QUrl("ws://127.0.0.1:7071"), bool connectNow = true,
                     QObject *parent = nullptr);

    QString state() const { return m_state; }
    QString speech() const { return m_speech; }
    QString heard() const { return m_heard; }
    QString tool() const { return m_tool; }
    bool connected() const { return m_connected; }
    bool demo() const { return m_demo; }

    // Applies one message from voiced. Public so tests can feed it directly.
    void handleMessage(const QString &message);

    // Space steps through every state with sample text, to preview the face
    // without talking to it. Any message from voiced ends the preview.
    Q_INVOKABLE void stepDemo();

signals:
    void stateChanged();
    void speechChanged();
    void heardChanged();
    void toolChanged();
    void connectedChanged();
    void demoChanged();

private:
    void setState(const QString &state);
    void setSpeech(const QString &speech);
    void setHeard(const QString &heard);
    void setTool(const QString &tool);
    void setConnected(bool connected);

    QUrl m_url;
    QWebSocket m_socket;
    QTimer m_retry;
    QString m_state = "idle";
    QString m_speech, m_heard, m_tool;
    bool m_connected = false;
    bool m_demo = false;
    int m_demoStep = -1;
};
