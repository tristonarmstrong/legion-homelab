// luna: Luna's face. A full-screen animated face that shows what Luna, the
// family's voice assistant, is doing: asleep, listening, thinking or talking.

#include <QDir>
#include <QElapsedTimer>
#include <QFile>
#include <QGuiApplication>
#include <QIcon>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QTimer>
#include <QUrl>

#include "backend.h"
#include "theme.h"
#include "translator.h"

int main(int argc, char *argv[]) {
    // `luna snapshot <state> <out.png>` renders the face offscreen in one demo
    // state and saves it, so the face can be checked without a display.
    const bool snapshot = argc >= 4 && QString::fromLocal8Bit(argv[1]) == "snapshot";
    // `luna record <dir>` plays a short scripted conversation offscreen and saves
    // every frame plus an ffmpeg concat list with each frame's real duration.
    const bool record = argc >= 3 && QString::fromLocal8Bit(argv[1]) == "record";
    if (snapshot || record)
        qputenv("QT_QPA_PLATFORM", "offscreen");

    QGuiApplication app(argc, argv);
    app.setApplicationName("luna");
    app.setApplicationVersion("0.1.0");

    // Matches the window to luna.desktop, so the compositor (Wayland app_id)
    // and the launcher pick up the installed icon.
    app.setDesktopFileName("luna");
    app.setWindowIcon(QIcon::fromTheme("luna"));

    QQuickStyle::setStyle("Material");

    // Installed before anything is translated, and every qsTr() binding is
    // re-evaluated when the Omarchy language changes.
    Translator translator;
    app.installTranslator(&translator);

    Theme theme;
    Backend backend(QUrl("ws://127.0.0.1:7071"), !snapshot && !record);

    QQmlApplicationEngine engine;
    QObject::connect(&translator, &Translator::changed, &engine, &QQmlEngine::retranslate);
    engine.rootContext()->setContextProperty("translator", &translator);
    engine.rootContext()->setContextProperty("theme", &theme);
    engine.rootContext()->setContextProperty("backend", &backend);
    engine.rootContext()->setContextProperty("snapshotMode", snapshot || record);
    engine.rootContext()->setContextProperty("snapshotAsleep", false);
    // Recording a promo: no random blinks or idle hint, so every take matches the script.
    engine.rootContext()->setContextProperty("recordMode", record);
    engine.load(QUrl("qrc:/Main.qml"));
    if (engine.rootObjects().isEmpty())
        return 1;

    if (snapshot) {
        // "asleep" is idle after Luna has dozed off; the face handles that itself.
        const bool asleep = QString::fromLocal8Bit(argv[2]) == "asleep";
        const QString wanted = asleep ? QStringLiteral("idle") : QString::fromLocal8Bit(argv[2]);
        engine.rootContext()->setContextProperty("snapshotAsleep", asleep);
        const QString out = QString::fromLocal8Bit(argv[3]);
        // Step the demo at least once so the face shows the preview, not "voiced isn't running".
        for (int i = 0; i < 6 && (i == 0 || backend.state() != wanted); ++i)
            backend.stepDemo();
        auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
        // Captured by value: `out` and `window` live only in this block.
        QTimer::singleShot(1800, &app, [window, out] {
            if (!window->grabWindow().save(out))
                qWarning("could not save %s", qPrintable(out));
            QCoreApplication::quit();
        });
    }

    if (record) {
        const QString dir = QString::fromLocal8Bit(argv[2]);
        QDir().mkpath(dir);
        // The promo: asleep, "Hey Luna!", listening, thinking, answering, smiling.
        struct Cue { int ms; const char *state, *speech, *heard, *tool; };
        static const Cue cues[] = {
            {0, "idle", "", "", ""},
            {950, "wake", "", "", ""},
            {1150, "listening", "", "", ""},
            {2850, "thinking", "", "Pull up a pancake recipe.", ""},
            {3000, "thinking", "", "Pull up a pancake recipe.", "web_search"},
            {3400, "speaking", "Pancakes are up on the screen!", "Pull up a pancake recipe.", ""},
            {5400, "idle", "", "", ""},
        };
        const int totalMs = 6600;
        backend.showScripted("idle", {}, {}, {}); // asleep, and not "voiced isn't running"
        for (const Cue &c : cues)
            QTimer::singleShot(c.ms, &app, [&backend, c] {
                backend.showScripted(QString::fromUtf8(c.state), QString::fromUtf8(c.speech),
                                     QString::fromUtf8(c.heard), QString::fromUtf8(c.tool));
            });

        auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
        auto *clock = new QElapsedTimer;
        auto *list = new QStringList;
        auto *last = new qint64(0);
        auto *frames = new QTimer(&app);
        frames->setInterval(1000 / 30);
        clock->start();
        QObject::connect(frames, &QTimer::timeout, &app, [=] {
            const qint64 now = clock->elapsed();
            const QString name = QString("f%1.png").arg(list->size() / 2, 5, 10, QChar('0'));
            window->grabWindow().save(dir + "/" + name);
            if (!list->isEmpty())
                list->append(QString("duration %1").arg((now - *last) / 1000.0, 0, 'f', 4));
            list->append(QString("file '%1'").arg(name));
            *last = now;
            if (now >= totalMs) {
                frames->stop();
                list->append(QString("duration %1").arg(1 / 30.0, 0, 'f', 4));
                QFile out(dir + "/frames.txt");
                if (out.open(QIODevice::WriteOnly))
                    out.write(list->join('\n').toUtf8() + '\n');
                QCoreApplication::quit();
            }
        });
        frames->start();
    }

    return app.exec();
}
