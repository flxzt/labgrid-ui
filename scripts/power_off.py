#!/usr/bin/env python3

# SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
#
# SPDX-License-Identifier: GPL-3.0-or-later

import os
from labgrid import Environment

lg_env = os.environ["LG_ENV"]
env = Environment(lg_env)
target = env.get_target("main")
power = target.get_driver("PowerProtocol")
power.off()
