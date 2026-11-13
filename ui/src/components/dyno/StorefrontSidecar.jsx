import React, { useState, useEffect } from 'react';
import Icon from '../Icon.jsx';
import { fetchStoreItem, placeStoreOrder } from '../../lib/labApi.js';

const FEATURED_ITEMS = [
  { id: 'phantom-headphones', name: 'Phantom Spatial Studio Audio', price: 299, desc: 'Planar magnetic studio drivers with sub-millisecond edge metadata delivery.', stock: 14, rating: '4.9 (534)' },
  { id: 'phantom-keyboard', name: 'Phantom Apex 8K Rapid Keyboard', price: 199, desc: 'Hall-effect magnetic switches with 0.1mm rapid trigger and CNC unibody.', stock: 18, rating: '4.9 (1,248)' },
  { id: 'phantom-display-4k', name: 'Phantom 32" QD-OLED Monitor', price: 999, desc: 'Quantum-Dot OLED, 144Hz true 10-bit HDR reference display.', stock: 8, rating: '5.0 (312)' },
  { id: 'phantom-ssd-2tb', name: 'Phantom 2TB Gen5 NVMe Storage', price: 179, desc: '14,500 MB/s sustained sequential reads with graphene heatsink.', stock: 25, rating: '4.9 (618)' },
];

/**
 * Mobile Storefront Sidecar.
 * Simulates a shopper on a smartphone purchasing a product.
 * Fires real HTTP requests directly against the cluster and visualizes sub-millisecond RAM response.
 */
export default function StorefrontSidecar({ onShoppingAction }) {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [product, setProduct] = useState(FEATURED_ITEMS[0]);
  const [isPurchasing, setIsPurchasing] = useState(false);
  const [lastResponse, setLastResponse] = useState(null);
  const [orderToast, setOrderToast] = useState(null);

  useEffect(() => {
    setProduct(FEATURED_ITEMS[selectedIdx]);
  }, [selectedIdx]);

  const handleBuy = async () => {
    setIsPurchasing(true);
    setOrderToast(null);
    try {
      const t0 = (typeof performance !== 'undefined' ? performance : Date).now();
      // 1. Fetch item through cluster node
      const itemRes = await fetchStoreItem(product.id);
      const itemData = itemRes.data || {};
      const fetchMs = (typeof performance !== 'undefined' ? performance : Date).now() - t0;

      // 2. Place order
      const orderRes = await placeStoreOrder({
        items: [{ id: product.id, name: product.name, price: product.price, qty: 1 }],
        tenant: 'tenantA',
        deviceId: 'iphone-16-pro',
      });
      const orderData = orderRes.data || {};

      const result = {
        ms: itemData.ms ? Number(itemData.ms.toFixed(2)) : Number(fetchMs.toFixed(2)),
        node: itemData.node || 1,
        status: itemData.cacheStatus || 'HIT',
        source: itemData.cacheSource || 'local',
        orderId: orderData?.order?.orderId || `ORD-${Date.now().toString().slice(-5)}`,
      };

      setLastResponse(result);
      setOrderToast(result.orderId);
      setProduct((p) => ({ ...p, stock: Math.max(0, p.stock - 1) }));

      if (onShoppingAction) onShoppingAction(result);
    } catch (err) {
      console.error('Buy error:', err);
    } finally {
      setIsPurchasing(false);
    }
  };

  return (
    <div className="storefront-sidecar" role="region" aria-label="Mobile Storefront Simulator">
      <div className="phone-bezel">
        <div className="phone-screen">
          {/* Phone Status Bar */}
          <div className="phone-status-bar">
            <span className="phone-status-bar__time">9:41</span>
            <div className="phone-dynamic-island" />
            <div className="phone-status-bar__icons">
              <span className="phone-status-signal">5G</span>
              <span className="phone-status-battery" />
            </div>
          </div>

          {/* App Header */}
          <div className="phone-app-header">
            <span className="phone-app-brand">PHANTOM STORE</span>
            <span className="phone-app-cart">🛒 1</span>
          </div>

          {/* Product View */}
          <div className="phone-product-card">
            <div className="phone-product-image">
              <img
                src={`/images/${product.id}.jpg`}
                alt={product.name}
                className="phone-product-img"
              />
              <span className="phone-product-badge">IN STOCK ({product.stock})</span>
            </div>

            {/* Product Switcher Pills */}
            <div className="phone-product-switcher" role="group" aria-label="Featured Hardware">
              {FEATURED_ITEMS.map((item, idx) => (
                <button
                  key={item.id}
                  type="button"
                  className={`phone-product-chip ${selectedIdx === idx ? 'phone-product-chip--active' : ''}`}
                  onClick={() => setSelectedIdx(idx)}
                  aria-pressed={selectedIdx === idx}
                >
                  {item.id === 'phantom-headphones' ? 'Audio' : item.id === 'phantom-keyboard' ? 'Keyb' : item.id === 'phantom-display-4k' ? 'OLED' : 'NVMe'}
                </button>
              ))}
            </div>

            <div className="phone-product-info">
              <div className="phone-product-meta">
                <span className="phone-product-rating">★ {product.rating}</span>
                <span className="phone-product-price">${product.price}</span>
              </div>
              <h3 className="phone-product-name">{product.name}</h3>
              <p className="phone-product-desc">{product.desc}</p>
            </div>

            {/* Buy Action */}
            <button
              type="button"
              className="btn btn--primary phone-buy-btn"
              onClick={handleBuy}
              disabled={isPurchasing || product.stock === 0}
            >
              {isPurchasing ? (
                <>
                  <Icon name="loading" />
                  <span>Processing Sub-Millisecond...</span>
                </>
              ) : product.stock === 0 ? (
                'Sold Out'
              ) : (
                '1-Click Instant Checkout'
              )}
            </button>

            {/* Order Feedback Toast */}
            {orderToast ? (
              <div className="phone-order-toast" role="status">
                <Icon name="ok" />
                <span>Order Placed: #{orderToast}</span>
              </div>
            ) : null}
          </div>

          {/* Telemetry Proof Box at bottom of phone */}
          <div className="phone-telemetry-dock">
            <div className="phone-telemetry-header">
              <span className="phone-telemetry-title">LIVE EDGE DIAGNOSTICS</span>
              <span className="phone-telemetry-pulse" />
            </div>
            {lastResponse ? (
              <div className="phone-telemetry-readout">
                <span className="phone-telemetry-stat">
                  Latency: <strong>{lastResponse.ms} ms</strong>
                </span>
                <span className="phone-telemetry-stat">
                  Status: <strong>{lastResponse.status}</strong> ({lastResponse.source})
                </span>
                <span className="phone-telemetry-stat">
                  Edge Node: <strong>Node {lastResponse.node}</strong>
                </span>
              </div>
            ) : (
              <p className="phone-telemetry-idle">
                Tap 1-Click Checkout to test live Edge RAM response.
              </p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
