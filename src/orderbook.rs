// orderbook.rs

use std::usize;
#[allow(unused_imports)]
use std::collections::BTreeMap;
use crate::interfaces::{OrderBook, Price, Quantity, Side, Update};


const CAP: usize = 4096;
const CAP_MASK: usize = CAP - 1;
const HALF_CAP: i64 = (CAP / 2) as i64;
const CAP_I64: i64 = CAP as i64;

pub struct OrderBookImpl {
    bids: [Quantity; CAP],
    asks: [Quantity; CAP],
    anchor_price: Price,
    best_bid_idx: usize,
    best_ask_idx: usize,
    total_bid_quantity: Quantity,
    total_ask_quantity: Quantity,
}



impl OrderBook for OrderBookImpl {
    fn new() -> Self {
        OrderBookImpl {
            bids: [0; CAP],
            asks: [0; CAP],
            anchor_price: 10000,
            best_bid_idx: 0,
            best_ask_idx: CAP_MASK,
            total_ask_quantity: 0,
            total_bid_quantity: 0,
        }
    }

    #[inline(always)]
    fn apply_update(&mut self, update: Update) {
        match update {
            Update::Set { price, quantity, side } => {
                let index = (price.wrapping_sub(self.anchor_price) as usize) & CAP_MASK;

                let (book, best_idx, total_qty, is_bid) = match side {
                    Side::Bid => (&mut self.bids, &mut self.best_bid_idx, &mut self.total_bid_quantity, true),
                    Side::Ask => (&mut self.asks, &mut self.best_ask_idx, &mut self.total_ask_quantity, false),
                };

                
                let old_quantity = unsafe { *book.get_unchecked(index) };

                if quantity > 0 {
                    unsafe { *book.get_unchecked_mut(index) = quantity };

                    if old_quantity == 0 {
                        *total_qty += quantity;
                    } else {
                        *total_qty = *total_qty - old_quantity + quantity;
                    }

                    
                    if *total_qty == quantity {
                        *best_idx = index;
                    } else if is_bid {
                         if index.wrapping_sub(*best_idx) & CAP_MASK < (CAP / 2) {
                             *best_idx = index;
                         }
                    } else {
                         if (*best_idx).wrapping_sub(index) & CAP_MASK < (CAP / 2) {
                             *best_idx = index;
                         }
                    }
                } else if old_quantity > 0 {
                    unsafe { *book.get_unchecked_mut(index) = 0 };
                    *total_qty -= old_quantity;

                    if index == *best_idx {
                        OrderBookImpl::recalculate_best_index(side, best_idx, book);
                    }
                }
            }

            Update::Remove { price, side } => {
                let index = (price.wrapping_sub(self.anchor_price) as usize) & CAP_MASK;
                
                let (book, best_idx, total_qty) = match side {
                    Side::Bid => (&mut self.bids, &mut self.best_bid_idx, &mut self.total_bid_quantity),
                    Side::Ask => (&mut self.asks, &mut self.best_ask_idx, &mut self.total_ask_quantity),
                };

                let removed_quantity = unsafe { *book.get_unchecked(index) };

                if removed_quantity > 0 {
                    unsafe { *book.get_unchecked_mut(index) = 0 };
                    *total_qty -= removed_quantity;
                    
                    if index == *best_idx {
                        OrderBookImpl::recalculate_best_index(side, best_idx, book);
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn get_spread(&self) -> Option<Price> {
        if self.total_bid_quantity == 0 || self.total_ask_quantity == 0 {
            None
        } else {
            let bid = self.index_to_price(self.best_bid_idx);
            let ask = self.index_to_price(self.best_ask_idx);
            Some(ask - bid)
        }
    }

    #[inline(always)]
    fn get_best_bid(&self) -> Option<Price> {
        if self.total_bid_quantity == 0 { None } else {
            Some(self.index_to_price(self.best_bid_idx))
        }
    }

    #[inline(always)]
    fn get_best_ask(&self) -> Option<Price> {
        if self.total_ask_quantity == 0 { None } else {
            Some(self.index_to_price(self.best_ask_idx))
        }
    }

    #[inline(always)]
    fn get_quantity_at(&self, price: Price, side: Side) -> Option<Quantity> {
        let index = (price.wrapping_sub(self.anchor_price) as usize) & CAP_MASK;
        let qty = unsafe {
            match side {
                Side::Bid => *self.bids.get_unchecked(index),
                Side::Ask => *self.asks.get_unchecked(index),
            }
        };
        if qty > 0 { Some(qty) } else { None }
    }

    fn get_top_levels(&self, side: Side, n: usize) -> Vec<(Price, Quantity)> {
        let mut result = Vec::with_capacity(n);
        let book = match side { Side::Bid => &self.bids, Side::Ask => &self.asks };
        match side {
            Side::Bid => {
                for i in (0..CAP).rev() {
                    let qty = unsafe { *book.get_unchecked(i) };
                    if qty > 0 {
                        result.push((self.index_to_price(i), qty));
                        if result.len() >= n { break; }
                    }
                }
            }
            Side::Ask => {
                for i in 0..CAP {
                    let qty = unsafe { *book.get_unchecked(i) };
                    if qty > 0 {
                        result.push((self.index_to_price(i), qty));
                        if result.len() >= n { break; }
                    }
                }
            }
        }
        result
    }

    #[inline(always)]
    fn get_total_quantity(&self, side: Side) -> Quantity {
        match side {
            Side::Bid => self.total_bid_quantity,
            Side::Ask => self.total_ask_quantity,
        }
    }
}



impl OrderBookImpl {
    #[inline(always)]
    fn index_to_price(&self, index: usize) -> Price {
        
        let offset = index as i64;
        let adjustment = if offset > HALF_CAP { -CAP_I64 } else { 0 };
        self.anchor_price.wrapping_add(offset).wrapping_add(adjustment)
    }

    fn recalculate_best_index(side: Side, best_idx: &mut usize, book: &[Quantity; CAP]) {
        match side {
            Side::Bid => {
                for i in (0..CAP).rev() {
                    if unsafe { *book.get_unchecked(i) } > 0 { *best_idx = i; return; }
                }
                *best_idx = 0;
            }
            Side::Ask => {
                for i in 0..CAP {
                    if unsafe { *book.get_unchecked(i) } > 0 { *best_idx = i; return; }
                }
                *best_idx = CAP_MASK;
            }
        }
    }
    
    #[allow(dead_code)]
    fn price_to_index(&self, price: Price) -> usize {
        (price.wrapping_sub(self.anchor_price) as usize) & CAP_MASK
    }
    #[allow(dead_code)]
    fn is_in_range(&self, price: Price) -> bool {
        (price - self.anchor_price).abs() < HALF_CAP
    }
    #[allow(dead_code)]
    fn recenter_anchor(&mut self, _new_price: Price) {}
}