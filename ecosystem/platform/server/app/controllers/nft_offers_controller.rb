# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NftOffersController < ApplicationController
  before_action :authenticate_user!, only: %i[create]

  def show
    store_location_for(:user, request.path)
    @nft_offer = NftOffer.find(params[:slug])
    @wallet = current_user&.wallets&.where(network: @nft_offer.network)&.first ||
              Wallet.new(network: @nft_offer.network, challenge: 24.times.map { rand(10) }.join)

    @transaction_hash = params[:txn]

    return render :minted if @transaction_hash.is_a?(String) && @transaction_hash.match?(/^0x[0-9a-f]{64}$/)

    @transaction_hash = nil

    @steps = [
      sign_in_step,
      connect_wallet_step,
      claim_nft_step
    ].map do |h|
      # rubocop:disable Style/OpenStructUse
      OpenStruct.new(**h)
      # rubocop:enable Style/OpenStructUse
    end
    first_incomplete = @steps.index { |step| !step.completed }
    @steps[first_incomplete + 1..].each { |step| step.disabled = true } if first_incomplete
  end

  def update
    @nft_offer = NftOffer.find(params[:slug])
    @wallet = current_user.wallets.where(network: @nft_offer.network).first!

    result = NftClaimer.new.claim_nft(
      nft_offer: @nft_offer,
      wallet: @wallet
    )

    render json: {
      wallet_name: @wallet.wallet_name,
      module_address: @nft_offer.module_address,
      message: result.message,
      signature: result.signature
    }
  rescue NftClaimer::AccountNotFoundError
    render json: { error: 'account_not_found' }
  end

  private

  def sign_in_step
    @login_dialog = DialogComponent.new(id: 'login_dialog')
    completed = user_signed_in?
    {
      name: :sign_in,
      completed:
    }
  end

  def connect_wallet_step
    completed = user_signed_in? && @wallet.persisted?
    {
      name: :connect_wallet,
      completed:
    }
  end

  def claim_nft_step
    completed = false
    {
      name: :claim_nft,
      completed:
    }
  end
end
