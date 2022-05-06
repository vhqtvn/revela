# frozen_string_literal: true

class WelcomeController < ApplicationController
  def index
    redirect_to overview_index_path if user_signed_in?
  end

  def it1; end
end
