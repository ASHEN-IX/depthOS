// Read-only Wayland inventory. No surfaces, activation or window-control requests.
#include <QGuiApplication>
#include <QTimer>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <cstdio>
#include <KWayland/Client/connection_thread.h>
#include <KWayland/Client/registry.h>
#include <KWayland/Client/plasmawindowmanagement.h>
using namespace KWayland::Client;
int main(int argc,char **argv) {
 QGuiApplication::setDesktopSettingsAware(false);
 QGuiApplication app(argc,argv);
 auto connection=ConnectionThread::fromApplication(&app);
 if(!connection)return 2;
 Registry registry;
 PlasmaWindowManagement *management=nullptr;
 QObject::connect(&registry,&Registry::plasmaWindowManagementAnnounced,&app,[&](quint32 name,quint32 version){management=registry.createPlasmaWindowManagement(name,version,&registry);});
 registry.create(connection);registry.setup();
 QTimer::singleShot(1200,&app,[&]{
  QJsonArray windows;
  if(management)for(auto w:management->windows()) {
   if(w->uuid().isEmpty() || !w->pid())continue;
   windows.append(QJsonObject{{"uuid",QString::fromUtf8(w->uuid())},{"pid",int(w->pid())}});
  }
  if(windows.isEmpty()){app.exit(3);return;}
  auto data=QJsonDocument(windows).toJson(QJsonDocument::Compact);
  std::fwrite(data.constData(),1,data.size(),stdout);app.exit(0);
 });
 return app.exec();
}
