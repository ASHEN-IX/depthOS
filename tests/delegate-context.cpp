// Offscreen regression probe, not a KWin integration test.
// Match v6.8.1 QuickSceneEffect::addScreen(): create the production delegate
// using an incubator and a NEW child of its creation context.
#include <QGuiApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QQmlComponent>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQmlIncubator>
#include <QQuickItem>
#include <QVariant>
#include <memory>

int main(int argc, char **argv)
{
    QGuiApplication app(argc, argv);
    if (argc != 3) {
        qCritical("Usage: delegate-context IMPORT_ROOT EFFECT_QML");
        return 2;
    }
    // Create before the engine scans this temporary directory: Qt also caches
    // directory entries, so creating a new QML filename later is a different test.
    const QString cachePath = QFileInfo(QString::fromLocal8Bit(argv[2])).dir().filePath("CacheProbe.qml");
    const auto writeRevision = [&cachePath](const QByteArray &revision) {
        QFile file(cachePath);
        if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
            return false;
        }
        const QByteArray data = "import QtQml\nQtObject { property string revision: \"" + revision + "\" }\n";
        return file.write(data) == data.size();
    };
    if (!writeRevision("before")) {
        qCritical("Cannot write temporary cache fixture");
        return 1;
    }
    QQmlEngine engine;
    bool qmlWarnings = false;
    QObject::connect(&engine, &QQmlEngine::warnings, [&qmlWarnings](const QList<QQmlError> &) {
        qmlWarnings = true;
    });
    engine.addImportPath(QString::fromLocal8Bit(argv[1]));
    QQmlComponent component(&engine, QUrl::fromLocalFile(QString::fromLocal8Bit(argv[2])));
    std::unique_ptr<QObject> effect(component.create());
    if (!effect) {
        qCritical() << component.errors();
        return 1;
    }
    auto *delegate = effect->property("delegate").value<QQmlComponent *>();
    if (!delegate) {
        qCritical("Missing production delegate");
        return 1;
    }
    QQmlContext *parentContext = delegate->creationContext();
    if (!parentContext) {
        parentContext = qmlContext(effect.get());
    }
    if (!parentContext) {
        parentContext = engine.rootContext();
    }
    QQmlContext screenContext(parentContext);
    QQmlIncubator incubator(QQmlIncubator::Asynchronous);
    incubator.setInitialProperties({{"width", 1920}, {"height", 1080}});
    delegate->create(incubator, &screenContext);
    incubator.forceCompletion();
    if (!incubator.isReady()) {
        qCritical() << "KWin-style child-context delegate creation FAILED:" << incubator.errors();
        return 1;
    }
    std::unique_ptr<QObject> view(incubator.object());
    auto *item = qobject_cast<QQuickItem *>(view.get());
    if (qmlWarnings || !item || item->width() != 1920 || item->height() != 1080) {
        qCritical("Delegate type/initial geometry mismatch");
        return 1;
    }
    qInfo("KWin-style child-context delegate creation: PASS (mock KWin types)");

    // KWin also retains its scripting QQmlEngine across effect unload/load.
    // Reproduce why replacing a QML file at the same URL does not refresh it.
    const auto readRevision = [&cachePath](QQmlEngine &targetEngine) {
        QQmlComponent cached(&targetEngine, QUrl::fromLocalFile(cachePath));
        QElapsedTimer wait;
        wait.start();
        while (cached.isLoading() && wait.elapsed() < 5000) {
            QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
        }
        std::unique_ptr<QObject> instance(cached.create());
        if (!instance) {
            qCritical() << cached.errors();
        }
        return instance ? instance->property("revision").toString() : QString();
    };
    if (readRevision(engine) != QStringLiteral("before")
        || !writeRevision("after-update") || readRevision(engine) != QStringLiteral("before")) {
        qCritical("Shared-engine cache reproduction did not match the live failure");
        return 1;
    }
    QQmlEngine freshEngine;
    if (readRevision(freshEngine) != QStringLiteral("after-update")) {
        qCritical("Fresh engine failed to load the updated file");
        return 1;
    }
    qInfo("Shared-engine same-URL reload stays cached; fresh engine sees update: confirmed");
    // Validate the chosen iteration strategy with the OLD engine: create a
    // new package directory only after it has already cached the original URL.
    const QString revisionDir = QFileInfo(cachePath).dir().filePath("new-package-revision");
    if (!QDir().mkpath(revisionDir)) {
        qCritical("Cannot create temporary versioned package");
        return 1;
    }
    for (const auto &entry : QList<QPair<QString, QByteArray>>{
             {QStringLiteral("Token.qml"), "import QtQml\nQtObject { property string revision: \"new-directory\" }\n"},
             {QStringLiteral("main.qml"), "import QtQml\nQtObject { property Token token: Token {} }\n"}}) {
        QFile file(QDir(revisionDir).filePath(entry.first));
        if (!file.open(QIODevice::WriteOnly) || file.write(entry.second) != entry.second.size()) {
            return 1;
        }
    }
    QQmlComponent versioned(&engine, QUrl::fromLocalFile(QDir(revisionDir).filePath("main.qml")));
    QElapsedTimer versionWait;
    versionWait.start();
    while (versioned.isLoading() && versionWait.elapsed() < 5000) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
    }
    std::unique_ptr<QObject> versionedObject(versioned.create());
    QObject *token = versionedObject ? versionedObject->property("token").value<QObject *>() : nullptr;
    if (!token || token->property("revision").toString() != QStringLiteral("new-directory")) {
        qCritical() << "New package URL failed in existing engine:" << versioned.errors();
        return 1;
    }
    qInfo("New package URL and relative component load in existing cached engine: PASS");
    return 0;
}
