// luna: Luna's face. A full-screen animated face that shows what Luna, the
// family's voice assistant, is doing: asleep, listening, thinking or talking.

#include <QDir>
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
    if (snapshot)
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
    Backend backend(QUrl("ws://127.0.0.1:7071"), !snapshot);

    QQmlApplicationEngine engine;
    QObject::connect(&translator, &Translator::changed, &engine, &QQmlEngine::retranslate);
    engine.rootContext()->setContextProperty("translator", &translator);
    engine.rootContext()->setContextProperty("theme", &theme);
    engine.rootContext()->setContextProperty("backend", &backend);
    engine.rootContext()->setContextProperty("snapshotMode", snapshot);
    engine.load(QUrl("qrc:/Main.qml"));
    if (engine.rootObjects().isEmpty())
        return 1;

    if (snapshot) {
        const QString wanted = QString::fromLocal8Bit(argv[2]);
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

    return app.exec();
}
