# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

Rails.application.routes.draw do
  devise_for :users, {
    controllers: {
      omniauth_callbacks: 'users/omniauth_callbacks',
      sessions: 'users/sessions'
    }
  }
  ActiveAdmin.routes(self)

  # Define your application routes per the DSL in https://guides.rubyonrails.org/routing.html

  namespace :user do
    root to: redirect('/it2') # creates user_root_path, where users go after confirming email
  end

  # Settings
  get 'settings', to: redirect('/settings/profile')
  get 'settings/profile'
  patch 'settings/profile', to: 'settings#profile_update'
  get 'settings/connections'
  delete 'settings/connections', to: 'settings#connections_delete'

  # Discourse SSO
  get 'discourse/sso', to: 'discourse#sso'

  # KYC routes
  get 'onboarding/kyc_redirect', to: 'onboarding#kyc_redirect'
  get 'onboarding/kyc_callback', to: 'onboarding#kyc_callback'

  get 'onboarding/email'
  get 'onboarding/email_success'
  post 'onboarding/email', to: 'onboarding#email_update'

  get 'health', to: 'health#health'

  resources :it2_profiles, except: %i[index destroy]
  resources :it2_surveys, except: %i[index destroy]
  resource :it2, only: %i[show]

  resources :nfts, only: %i[show update]
  resources :nft_offers, only: %i[show update]

  get 'nft-nyc', to: 'nft_nyc#show'

  get 'leaderboard/it1', to: redirect('/it1')

  get 'it1', to: 'leaderboard#it1'
  root 'static_page#community'
end
