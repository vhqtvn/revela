# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ProjectCardComponent < ViewComponent::Base
  def initialize(project:, **rest)
    @project = project
    @rest = rest
    @rest[:class] = [
      'rounded-lg overflow-hidden cursor-pointer hover:brightness-105',
      @rest[:class]
    ]
    @rest[:onclick] = 'this.querySelector("a").click()'
  end
end
