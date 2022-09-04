# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# An offer for a user to claim an NFT (e.g. for promotional purposes).
class NftOffer
  include ActiveModel::Model

  attr_accessor :slug, :network, :module_address, :private_key

  def self.find(slug)
    case slug
    when 'aptos-zero'
      NftOffer.new(
        slug: 'aptos-zero',
        network: 'devnet',
        module_address: ENV.fetch('APTOS_ZERO_NFT_MODULE_ADDRESS'),
        private_key: ENV.fetch('APTOS_ZERO_NFT_PRIVATE_KEY')
      )
    else
      raise ActiveRecord::RecordNotFound
    end
  end

  def private_key_bytes
    [private_key[2..]].pack('H*')
  end

  def persisted?
    true
  end

  def to_key
    [slug]
  end
end
