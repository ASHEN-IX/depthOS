// SPDX-License-Identifier: GPL-2.0-or-later
// Runtime fallback for the exact installed libkwin ABI, never installed.
#include "backends/virtual/virtual_backend.h"
#include "depthos_authority_experiment.h"
#include "main.h"
#include "plugin.h"

namespace KWin
{
class PrivateDepthOSPlugin final : public Plugin
{
    DepthOSAuthorityExperiment m_authority;
};

class PrivateDepthOSFactory final : public PluginFactory
{
    Q_OBJECT
    Q_PLUGIN_METADATA(IID PluginFactory_iid FILE "private-plugin.json")
    Q_INTERFACES(KWin::PluginFactory)

public:
    std::unique_ptr<Plugin> create() const override
    {
        if (!kwinApp() || !qobject_cast<VirtualBackend *>(kwinApp()->outputBackend())) {
            return {};
        }
        return std::make_unique<PrivateDepthOSPlugin>();
    }
};
}

#include "private-plugin.moc"
