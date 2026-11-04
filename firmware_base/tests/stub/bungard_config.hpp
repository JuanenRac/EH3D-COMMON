// A stand-in for the node-specific settings header, so the shared code and its test build here: a node adds its own machine settings to Settings.
#pragma once
#include "../../core/node_settings.hpp"

namespace eh3d::config {
struct Settings : NodeSettings {};
}  // namespace eh3d::config
