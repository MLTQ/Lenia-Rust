use crate::lenia::{GrowthFuncType, KernelCoreType, LeniaParams};
use ndarray::Array2;

const MIN_SPECIES_WORLD_SIZE: usize = 128;
const MAX_SPECIES_WORLD_SIZE: usize = 512;
const WORLD_SIZE_GRANULARITY: usize = 32;

#[derive(Clone, Copy, Debug)]
pub struct SpeciesPreset {
    pub code: &'static str,
    pub name: &'static str,
    pub cname: &'static str,
    pub radius: usize,
    pub time_scale: usize,
    pub shell_weights: &'static str,
    pub mu: f64,
    pub sigma: f64,
    pub kernel_core_type: KernelCoreType,
    pub growth_func_type: GrowthFuncType,
    pub cells: &'static str,
}

#[derive(Clone, Debug)]
pub struct LoadedSpecies {
    pub params: LeniaParams,
    pub world: Array2<f64>,
    pub pattern_rows: usize,
    pub pattern_cols: usize,
}

impl SpeciesPreset {
    pub fn short_label(self) -> String {
        format!("{}  {}", self.code, self.name)
    }

    pub fn detail_label(self) -> String {
        format!(
            "{} | R={} T={} b={} kernel={} growth={}",
            self.cname,
            self.radius,
            self.time_scale,
            self.shell_weights,
            self.kernel_core_type.as_str(),
            self.growth_func_type.as_str()
        )
    }

    pub fn load(self) -> Option<LoadedSpecies> {
        let pattern = decode_rle_2d(self.cells)?;
        let shell_weights = parse_shell_weights(self.shell_weights)?;
        let params = LeniaParams::from_official_lenia(
            self.radius,
            self.time_scale,
            shell_weights,
            self.mu,
            self.sigma,
            self.kernel_core_type,
            self.growth_func_type,
        );
        let world_size = recommended_world_size(&pattern, self.radius);
        let world = centered_pattern_world(&pattern, world_size, world_size);

        Some(LoadedSpecies {
            params,
            world,
            pattern_rows: pattern.nrows(),
            pattern_cols: pattern.ncols(),
        })
    }
}

pub fn curated_species() -> &'static [SpeciesPreset] {
    &CURATED_SPECIES
}

fn parse_shell_weights(input: &str) -> Option<Vec<f64>> {
    let mut values = Vec::new();
    for token in input
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        values.push(parse_scalar(token)?);
    }
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn parse_scalar(input: &str) -> Option<f64> {
    if let Some((numerator, denominator)) = input.split_once('/') {
        let numerator = numerator.trim().parse::<f64>().ok()?;
        let denominator = denominator.trim().parse::<f64>().ok()?;
        if denominator.abs() <= f64::EPSILON {
            return None;
        }
        Some(numerator / denominator)
    } else {
        input.trim().parse::<f64>().ok()
    }
}

fn decode_rle_2d(source: &str) -> Option<Array2<f64>> {
    let mut rows = Vec::<Vec<f64>>::new();
    let mut current_row = Vec::<f64>::new();
    let mut count = String::new();
    let mut prefix = String::new();

    for ch in source
        .trim()
        .trim_end_matches('!')
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .chain(std::iter::once('$'))
    {
        if ch.is_ascii_digit() {
            count.push(ch);
            continue;
        }

        if matches!(ch, 'p'..='y' | '@') {
            prefix.clear();
            prefix.push(ch);
            continue;
        }

        let repeat = count
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(1);
        let token = if prefix.is_empty() {
            ch.to_string()
        } else {
            let mut token = prefix.clone();
            token.push(ch);
            token
        };

        if token == "$" {
            rows.push(std::mem::take(&mut current_row));
            for _ in 1..repeat {
                rows.push(Vec::new());
            }
        } else {
            let value = decode_cell_token(&token)? / 255.0;
            current_row.extend(std::iter::repeat(value).take(repeat));
        }

        count.clear();
        prefix.clear();
    }

    if rows.is_empty() {
        rows.push(current_row);
    }

    let width = rows.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let height = rows.len().max(1);
    let mut array = Array2::<f64>::zeros((height, width));

    for (row_index, row) in rows.iter().enumerate() {
        for (col_index, value) in row.iter().enumerate() {
            array[(row_index, col_index)] = *value;
        }
    }

    Some(array)
}

fn decode_cell_token(token: &str) -> Option<f64> {
    if matches!(token, "." | "b") {
        return Some(0.0);
    }
    if token == "o" {
        return Some(255.0);
    }

    let mut chars = token.chars();
    let first = chars.next()?;
    let second = chars.next();

    match second {
        None => Some((first as u32).checked_sub('A' as u32)? as f64 + 1.0),
        Some(second) => {
            let prefix = (first as u32).checked_sub('p' as u32)? as f64;
            let suffix = (second as u32).checked_sub('A' as u32)? as f64;
            Some(prefix * 24.0 + suffix + 25.0)
        }
    }
}

fn recommended_world_size(pattern: &Array2<f64>, radius: usize) -> usize {
    let padding = (radius.max(1) * 10).max(96);
    let desired = pattern.nrows().max(pattern.ncols()) + padding;
    let rounded = desired.div_ceil(WORLD_SIZE_GRANULARITY) * WORLD_SIZE_GRANULARITY;
    rounded.clamp(MIN_SPECIES_WORLD_SIZE, MAX_SPECIES_WORLD_SIZE)
}

fn centered_pattern_world(pattern: &Array2<f64>, rows: usize, cols: usize) -> Array2<f64> {
    let mut world = Array2::<f64>::zeros((rows, cols));
    let row_copy = pattern.nrows().min(rows);
    let col_copy = pattern.ncols().min(cols);
    let src_row_start = (pattern.nrows() - row_copy) / 2;
    let src_col_start = (pattern.ncols() - col_copy) / 2;
    let dst_row_start = (rows - row_copy) / 2;
    let dst_col_start = (cols - col_copy) / 2;

    for row in 0..row_copy {
        for col in 0..col_copy {
            world[(dst_row_start + row, dst_col_start + col)] =
                pattern[(src_row_start + row, src_col_start + col)];
        }
    }

    world
}

const CURATED_SPECIES: [SpeciesPreset; 8] = [
    SpeciesPreset {
        code: "O2u",
        name: "Orbium unicaudatus",
        cname: "球虫(單尾)",
        radius: 13,
        time_scale: 10,
        shell_weights: "1",
        mu: 0.15,
        sigma: 0.015,
        kernel_core_type: KernelCoreType::Polynomial,
        growth_func_type: GrowthFuncType::Polynomial,
        cells: "7.MD6.qL$6.pKqEqFURpApBRAqQ$5.VqTrSsBrOpXpWpTpWpUpCrQ$4.CQrQsTsWsApITNPpGqGvL$3.IpIpWrOsGsBqXpJ4.LsFrL$A.DpKpSpJpDqOqUqSqE5.ExD$qL.pBpTT2.qCrGrVrWqM5.sTpP$.pGpWpD3.qUsMtItQtJ6.tL$.uFqGH3.pXtOuR2vFsK5.sM$.tUqL4.GuNwAwVxBwNpC4.qXpA$2.uH5.vBxGyEyMyHtW4.qIpL$2.wV5.tIyG3yOxQqW2.FqHpJ$2.tUS4.rM2yOyJyOyHtVpPMpFqNV$2.HsR4.pUxAyOxLxDxEuVrMqBqGqKJ$3.sLpE3.pEuNxHwRwGvUuLsHrCqTpR$3.TrMS2.pFsLvDvPvEuPtNsGrGqIP$4.pRqRpNpFpTrNtGtVtStGsMrNqNpF$5.pMqKqLqRrIsCsLsIrTrFqJpHE$6.RpSqJqPqVqWqRqKpRXE$8.OpBpIpJpFTK!",
    },
    SpeciesPreset {
        code: "O2b",
        name: "Orbium bicaudatus",
        cname: "球虫(雙尾)",
        radius: 13,
        time_scale: 10,
        shell_weights: "1",
        mu: 0.15,
        sigma: 0.014,
        kernel_core_type: KernelCoreType::Polynomial,
        growth_func_type: GrowthFuncType::Polynomial,
        cells: "13.pK$14.qV$6.VpA.MpEpKpITqV$4.BpPpNrIrEqDpWpOpLpUqNvT$4.IqRrNsPsKqHJ3.GqOuC$4.TrLsTrPrLpS6.uUD$3.SpWqNrBqLpRqPqE6.vA$2.FpTpMLpHqPqHrVsPrS5.qUqA$K.pCpRG.ErFsRsVuSuPqN4.CrR$pA.pTU3.rWuBuRvXwTwKpF4.rCH$.tPqHH3.qFvAwUwVyJyKwNL2.DqLR$.pGsGA4.vPxSyDxE2yOuHS.XqJT$2.xIE4.sCyHyOvLvRyFxCsGpVpXqGP$2.VsU4.DxQyOvVuSwDwQuBrMqSqCF$3.vG5.tEyKwVvIvKvMtVrXqTpM$4.sU4.qFvDwMvNuUuDsUrKqDO$4.qCrDJ2.pPsKuGuHtOsQrNqKpC$5.pTqTpVpNqFrJsGsKrVrDqFpFD$6.QqCqJqPqVqXqRqHpOTC$8.LWpFpEXPG!",
    },
    SpeciesPreset {
        code: "OG2g",
        name: "Gyrorbium gyrans",
        cname: "旋球虫",
        radius: 13,
        time_scale: 10,
        shell_weights: "1",
        mu: 0.156,
        sigma: 0.0224,
        kernel_core_type: KernelCoreType::Polynomial,
        growth_func_type: GrowthFuncType::Polynomial,
        cells: "10.EL2QLE$7.TpU2qHqCpXpUpNpFL$4.JrVtTuKuPuKtLrXqTqHqCpPpDG$3.qWtDqRpKqEsMuXvBtGrApXpUpSpIO$2.rQrN4.pAuAvRtTrIpUpIpKpFO$.pSsM6.tJwFuNsPsFrVpPpDL$.uFB6.tJ2yO2yLyOyDsKL$pDuC6.pFxW3yOwIwD2xPqH$rNtV5.EsMxCyIyOwXtJsMtJwFuX$sHuSV3.EpDvOwFxEwQsRqR2qHsFvWE$rQvJsWpPQpKpSqCvEvBuCpD3.BpDtGrQ$pXuKvMuPtLsWsCrIuCtBrS6.qWrQ$EsKvEwXyBwLtVrVsCrDqH6.pXrG$.qHtVxJyOwQrQpNqJpPV6.qJqE$.JsUxMyOrX10.pFqRJ$2.rQxPwIpI9.pKqJT$2.qJxEuPpKB7.qCpP$2.EvOvMpPO5.TrGqH$3.sCyOqEpIOEBOqHqEsRtG$4.xMsMqJqCpXqJqRpIqOuCtBsF$5.xPrAqTqMpSE.rSsMrLqRqHV.TpS$6.vErDE2.VpPB$7.pIrNqHpKQ!",
    },
    SpeciesPreset {
        code: "OV2u",
        name: "Vagorbium undulatus",
        cname: "遊球虫",
        radius: 20,
        time_scale: 10,
        shell_weights: "1",
        mu: 0.2,
        sigma: 0.031,
        kernel_core_type: KernelCoreType::Polynomial,
        growth_func_type: GrowthFuncType::Polynomial,
        cells: "17.DWqD2qUqES$7.D2FDCBCHqGsTvBwQxL2xSxOwWvJsSpC$5.DJOQPNMPqWtQwHyAyHxLwAuSuGuTwBxLxXwNtAO$4.FQpB2pGpEpDqLsSvJxOyOxXvGrRR3.NrNvDxQxPuXqP$3.ESpJpSpXpWqCrMtOvRxWyOxWuOpV6.CpHtWxCxPvIrM$3.PpMqEqNqQqXsBtQvKxJ2yOvWqW8.GpXuNxFxBuWrJ$2.HpIqIrArIrPsJtLuQwGxWyOyHuHL9.XsPwExEwEtWqU$2.UqCrDrSsCsQtItXuWwIyDyOyDtK10.PsIvTwXwKuSsPqC$.GpMqUrTsJsUtItOtWuOwFyHyOyJtF10.LtIwMxAwDuUtFrHpJ$.PqErMsJsWtKtOtKtNuIwAyN2yOtVpF9.pRvKxRxCvPuFsUrOqIpCJ$.pBqRrXsStHtOtL2tEuDwE3yOvIrHpKK7.sKxWyOwUuStDrWrDqLpTpGR$DpLrCsHtAtJtMtGsXtDuHwR2yOyHwAsSqUpPO3.DqGpKvH2yOwCtMrSqQqEqApXpUpMJ$GpQrHsJsXtHtItBtAtMvByC2yOyAvHsNrFqBU3.qVrStD2yOyNvEsGqLpKpApFpQqCqApA$HpSrGsGsSsXtBsXtHuHwJ3yOxOuGrRqQpXQ2.qLqWsCwC2yOyIwTuVqXJ.JpEqAqFpI$HpPrArVsHsKsRtCtXvN4yOxCsGqJpTpOP.pRqAqPsAwW2yOyCwIvDuKqI2.PpUqDpJ$FpIqLrFrNrSsHtHuVxX4yOtGpQXSQRUpBpJpUrJxQ2yOxTwDvBuQtAW.HpMpUpC$CVpTqJqPqUrStNwA4yOuA6.BEISqExH2yOxPwDvEuUuBqW.BpCpHM$.KpApJpNpRrAtPyI3yOuHS10.WvQyOyNxNwHvLuTtQrO2.MN$.CJ2NMqAtQ3yOuVrMR11.pIsDvGvOvBuOuCtGrJ$2.A2CANtR2yOxQsQrHX10.CpPrJrWrXtE2tSsXqV$7.uIyLyOtLsKrIpJ10.LpUqWrErBrLsUtKsSqG$7.uCyAyMsUsIrMqCK9.TqAqP2qOqVrVsUsJpM$7.sNxExLsMsErTqSpHE7.KpJqEqJqGqEqNrHsBrMN$7.qBvUvJ3rVrJqKpEH5.IpApTqFqDpXpWqEqSrCqH$8.uBtJqPrBrNrQrHqMpNRKIKRpHpTqEqCpTpOpNpUqEqCR$8.rUrLOpNqHrCrMrLrAqKpWpRpSpWqGqEqCpRpLpHpG2pJS$8.pApW2.FpFqEqWrHrGrDqVqRqMqFpXpNpFXUSRI$9.P5.HpEpQ2qCpVpNpEVPJDC$19.B$17.CUS$16.OpOpSpXqD$17.GJLPS!",
    },
    SpeciesPreset {
        code: "2S1f",
        name: "Scutium serratus fluens",
        cname: "鋸盾虫(流)",
        radius: 15,
        time_scale: 10,
        shell_weights: "1/2,1",
        mu: 0.31,
        sigma: 0.0575,
        kernel_core_type: KernelCoreType::Polynomial,
        growth_func_type: GrowthFuncType::Polynomial,
        cells: "15.2BA$13.DQpCpFpCSJC$13.UpWqRrCqXqLpTpCLC$12.JpUrHsMtDtEsPrSqRpSWJC$11.DpQrItCuNvKvNvAtXsRrLqJpMVKC$9.DpFqOsEuBvWxLyHyGxNwHuVtJsCrAqFpMUF$7.pAqRsFtOuXwNyJ6yOxEvLtWsRrSqVpXpBG$6.qTtWwO12yOxPvUuKtHsIrHqCXD$5.rEwLyH14yOxRwCuTtPsMrEpTN$4.pXvIxQ16yOxMwBuStMsCqOpCD$4.tLwGyE9yOyLyM6yOxCvQuGsRrDpNI$3.rJuDwPyI5yOyJyBxQxExDxSyK5yOxXwKuTtErLpUM$2.pPrVuHwTyL3yOwKvCuRuMuCuAuWwMxVyN5yOxDvGtJrOpWO$.HqCsCuNxF2yOyAsApWpTqBqCqFrAsRvAxByH5yOxWvQtOrPpWN$.RqLsQvLyByOwKD6.OqUtLvVxQyL5yOwEtSrQpVM$.pGrLuEwXyOuRS8.pBrUuMwRyC5yOwRuCrTpVL$.qJtDvVyAsXqKE9.qItEvRxOyM4yOxFuKsApWL$WrUuOuHsOrVpRD9.pNsMvJxMyM4yOxMuQsEqAL$qAsFpQrGsDrQpUG9.qBtIwJyE5yOxLuRsFqAL$2.pFqUrXrWqLR8.IrUvOyA6yOwXuKsBpVJ$2.NqIrUsLrOpQF7.qCuFxO6yOyKvXtTrPpPG$3.pQrMsTsPqWXA6.sIwR7yOwHuKsSqWpDB$3.UqTsJsUrVqCP5.rCwF6yOyKvOtTsPrKpXO$4.pPrHsHsFrHqCpBKEDtFyI6yOyEuPsMrKqSqApAC$4.JpXrGrXsCrUrNrMvF9yOuDrHpVpIpCSE$5.QqDrFsAsRwI10yOuKqRP$6.VqCtHwTwSwRwUxIxXyJ3yOyNuTqQA$7.tPtVtR2tMtXuWvXwVxKxQxOuDqL$6.qCqNqPqOqPrArRsPtOuIuTuErPpG$9.HSpFpSqGqDS!",
    },
    SpeciesPreset {
        code: "2S2",
        name: "Discutium serratus",
        cname: "乙鋸盾虫",
        radius: 13,
        time_scale: 10,
        shell_weights: "5/12,1,1/2",
        mu: 0.272,
        sigma: 0.0375,
        kernel_core_type: KernelCoreType::Polynomial,
        growth_func_type: GrowthFuncType::Polynomial,
        cells: "11.BGE2.B2EGE$10.BTpFpKpPpXqCpUpIpAVQE$9.BTpKqErLsPtBsUsFrA2pXqJqCpFJB$6.LpKqHqJqHqTsKtQuI2uUuNtVtG3sRsCqTpKVJ$4.BpFrNtJ2tVtQuIvEuUvBvTwDwQvOvBvMvBuSuFtDrGpSpAE$5.pAsPuSvRwSxRxWwXvOvWxMyGyDvJuSwXwVvTvEuIsWqOpIJ$5.ErAuIwNxUyI2yOxWxJyB2yOxJwS2yOxEvOuUuSsMqJpKJ$3.BLpFqTtOxPxWvRwQ2yIxUtVuUyL4yOyDwIvTwQvWtVsPrLpNB$2.pXtDvBvRwDxP2yOuAtOwQxEwIrVsFxEyDyBvGtOwQyL3yOxMvTuNtBqHE$.EtJxW6yOuUrIrXuCuFtLtOvEwFvWsUrXwXyG2yOyByGxEvRuFrLQ$2.sF6yOyGvRpS.LrX2sKrVrS2tJtVvOwVxCvJvMxWxPuSsFpN$3.pSwA3yOvJuUtOpK2.pNqWqOT.rIsHsPsWsCsFsRwDyOxPsRpUB$4.BwQ2yOtB2pKB3.EJ.EpPqOqH2.pDuA3yOwNtBqMJ$3.EtQxEyGyOvE15.qOvTyI4yOwXtOpK$pFxHxPxW5yO15.pXtGvGwX4yOxMsH$qTyIxUxM5yOpAB15.rLxR4yOxCrX$.vJvRuIwNyB2yOxErApKB14.uU2yOxWtTpNE$4.qTuAwNwAtGsHrIpUQTpFQ9.J2yOxWtQ$5.pUuCwL2sFrQrD2qOrAqOpSpNpSpNL.ETpKwQ4yOuU$5.QtOwIrDqJqHqJqRrAqOqHqJqOrAqWqEpPqHrDrQxP4yOyBwStL$3.pPqWsKuFuXQ.JTpKpIOJVpKqE2qTqWrNsCsHuPyI2yBxRwXxJyDqT$2L2.qCrIsCsKG9.BpFpSpXqMrDrVuAvWuItJsPtTwXxM$6.TpU13.BGpFqTwAtGpP$24.uKuSsF$24.sH2sUrApA$24.pUpXqCqEpF$30.pF$30.J!",
    },
    SpeciesPreset {
        code: "O4dp",
        name: "Parorbium dividuus pedes",
        cname: "並球虫(分步)",
        radius: 25,
        time_scale: 10,
        shell_weights: "1",
        mu: 0.11,
        sigma: 0.011,
        kernel_core_type: KernelCoreType::Exponential,
        growth_func_type: GrowthFuncType::Exponential,
        cells: "54.D$53.MXWH$52.CpBpPpSpIL$52.OpRqIqOqJpQ2.AHN2POMA3.J$52.WqFrDrLrKqUpROXpBpH3pJpIXP$52.pIqTrQsGsJrWrApLpOpQ2pOpMpLpN2pMpBB.X$51.EpQrEsIsVtCsTrXpXpSpOpEVRPSXpOpTpGCrH$48.GNTpCpIrMsPtJtNtFsIqDpITKA4.MpIpXpLqR$46.GRpBpHpMpRpVrKsNtEtIsWrWqGMB7.CpGqDpMqT$44.DOpCpFpKpNpO2pPqJrNsEsIsCrFpU11.pLqKuD$44.OpFpKpJpBXUSRPqFrArHrGqSpUI10.ApVuB$43.NpHpMpCSIA4.pBqCqUrAqUqIpK11.IqWtC$13.AD28.pCpLpEM7.RqAqTrHrJrDqM12.pBwJ$12.ATVWC23.pB.IpKpOQ8.HqErDrVsDsCrPqT12.uDpJ$12.OpOpKpXpHG2.BGLN2ONE14.RpSpHB9.qQrTsLtAtBsSsA12.MuO$12.pDqIqEqIqLpLJNUVpE2pH2pIpCUF11.pIpAqAV10.rBsHtFtTuBtWtGrI12.vB$12.pRrBrCqXrNqP2pFpKpMpLpNpMpL2pNpPpHN.pV8.rPpGqEP10.rMsWtXuOuXvAuMtP12.rJqO$10.EJqArNsDrRsLrRqJpR2pOpHXUSTXpNpVpMMqO8.qEpKqHJ10.qQtLuPvKvLwBvRvBrE11.DtA$7.BJQVpCqIsAtDsMsXsOrHpQpLXPF4.MpHqCpRpD9.sKqJF10.pEuAvHwFwCwTwUwHuT12.sL$6.LVpBpGpLpOqIsHtM2tFsXrOpMSH7.CpHqHpMrT8.uKqPD11.uKvXwXwWxExUxKwOqK11.qSpL$5.QpBpL2pOpPpRpVrXtGtRtDsUrLpKC9.ApPqLtS8.sWqVE11.uJwLxNxUxNyMyHxTuF11.pJqL$4.MpEpNpOpMpJpHpGpHrBsLtDsWsKrGpO11.FqHsW8.OuEG11.sQxAyCyOxRyK2yOxOpG10.PqQ$3.DpDpPpNpATO3LpSrMsJsMsErIqCC11.RqXsO8.wKK11.qOxIyLyOxRxO3yOsQ10.JqL$G2.QpOpPWI5.UqRrUsIsHrTqXpP12.qBvX8.vFR11.KxG2yOxQwQxW2yOvPD9.HqGF$3.pFpRpAE7.qMrTsOsRsMrXqX12.IvM8.rFsE12.vD2yOxSwFwS2yOxQqR9.HqBH$2.E2pPL8.qOsDsXtItHsWsCG12.rHrX8.uV12.sR2yOyB2wFxSyOyMsW9.PpUG$.qSPpVpF9.qWsLtLuAuDtUtDrN12.GvU8.uO12.qJxUyOyLwNwDxDyNyJuNpF8.UpRD$.rUXqCS9.qKtAuCuOuUuQuEtF13.vL8.rOqH11.MvH2yOxCwGwSxTxVvIqW7.ApFpN$.qLpIqEJ9.ItIuNvFvMvLvDuIJ12.rTqC7.LsK12.sTxNyMxKwOwMxExIvMsDW6.NpMpC$2.rLqEA10.tRvCvUwFwGwBvJrQ12.IsW8.sCP11.qSvLxOxJwRwKwNwPvLsTqEH4.IpBpNR$2.uJqF11.tXvLwIwVxAwWwIuQ13.tJ8.pUqP11.pMtNwGwTwLwDwBvUvCtGrBpKLEFJWpMpKF$2.vFqG11.rWvXwXxJxPxOxFwKI12.sED7.BrAT10.TsAuSvVvUvPvKvEuMtHrQqHpKpBVpEpMpOV$2.tBqK11.pRwHxIxXyDyEyAxKrP12.qSpF8.pLqFB9.NqXtKuTvCuUuQuKtVtBsAqUqF2pQpRpPpGI$2.VtF12.wPxUyI3yOyFuT12.2pV8.AqApL9.NqJsItQuEuCtTtNtFsPrXrFqOqGqBpTpMQ$3.vV12.vHyD5yOxMpB11.pKqB9.NpVpBB7.RqCrOsS2tGtBsRsLsCrNrDqQqIpWpNTA$3.wC12.tKyJ6yOsC11.pDpW10.SpQpFK5.GpCqDrEsA2sKsGrXrPrJrBqQqIpUpLTE$3.tGK11.rP7yOuR11.pHpR11.SpOpJVM2KMXpNqHrArMrQrOrKrEqUqNqIpXpOpEQD$3.pIrP11.qAyB6yOwNqE10.pHpL12.NpHpMpIpGpEpGpNpXqJqT2rAqTqNqHqBpRpLpESLB$4.tM11.SwH2yOyMyC2yOxLsG9.ApOpA13.DUpIpOpNpQpVqCqG2qJqGqBpRpKpFXPKE$4.sP12.uP2yOyCxLxTyKxOtUpG8.NpNQ15.FSXpIpNpP2pQpOpKpEWQLHD$4.qAqH11.tDxQyNxPwXxAxMxGuTqU7.ApBpOF18.INQ2RPNKHD$4.CrJ11.rWwHxQxEwMwJwQwOvCsCpG6.NpIpB$5.qLpJ10.rBvAwTwOwAvSvVvUuWsUqJU3.AOpGpMO$5.OqKF9.qLtUvSvXvK2vDvCuLtCrHpUpAPNVpGpLpA$6.pTpN9.qDsVuRvEuTuKuIuGtVsXrSqNpUpIpJpLpMpHL$6.JpWpB7.HqBsDtRuGuCtRtMtKtEsOrTrAqIqApUpRpLR$7.UpSWC5.SqCrPsVtJtHtAsRsNsJsBrLrAqMqFpUpMU$8.pApNpDOEDGOpGqErGsDsMsLsFrXrRrNrIrAqPqHpTpLTD$9.WpOpKpBXpBpFpPqIrArMrQrOrJrEqWqRqNqIqApQpGRD$10.TpLpNpOpPpQpXqJqS2qXqTqNqIqEpXpRpNpHUNB$11.LXpMpQpSpXqEqHqIqGqBpTpNpJpGpBSNH$13.NVpEpKpNpOpNpKpHpCWRNKGA$15.GMPQPOMKIFC$!",
    },
    SpeciesPreset {
        code: "O6?",
        name: "Triparorbium(?)",
        cname: "三球虫",
        radius: 16,
        time_scale: 10,
        shell_weights: "1",
        mu: 0.114,
        sigma: 0.0112,
        kernel_core_type: KernelCoreType::Exponential,
        growth_func_type: GrowthFuncType::Exponential,
        cells: "60.CD$53.LpEM2.KXpLpNpOpHJrI$53.pQqJqOpOVpMpPpHVUpGpXpMrN$53.qArHsHsBqPpSVB4.pMqS$53.qEsJtBtPsEW7.QvV$52.SqAsKtOtAsCpM8.IuN$50.HpApQpTrBsFrMrRqOpC8.pSK$49.BpEpOpHRIqCrGqWrSrE9.tM$49.QpNV3.VqXrWsOtCsK9.rA$49.pJpB5.qWsWtNuRuPtB8.qS$48.GpPL5.qDtIvAvNwHwBqX7.IpN$47.pQJpT7.tNvOwTxMxUxGR7.qF$48.EqC7.pRwAxRxT2yOxH7.pU$48.sNqK8.vVyDyOxE2yOuK6.pPD$49.tSK7.pKyHyOwGvX2yOrK5.pPF$49.qQpS8.vNyOxDvCwIyDvHpV3.SpOB$38.BA10.wL8.pJyOyMvUvMwQwGsQpQQTpJpN$30.K4.KTpHpKpNpKTsE8.qO8.tHxXwTvNvPvNtSrKqDpVpWW$29.OpOpXpGBUpKpOpHWUpGqBpVqQ7.tK8.pDuQwLvNuVuKtKsAqXqIpQJ$29.SqSrHrNqJ2pQUA4.pNsH8.sJ8.rLuKuQtXtGsMrPqSpVT$29.OrEsItFsLqET7.RwL7.RqH7.pUsFtFsTsErKqPpUXC$29.JrItItEtIqS9.JuL7.pIpT5.BpLrBrWrQrBqHpLTE$27.AVpJqNsPtArXrBpB9.qCV7.UpVUHCHVpTqQqWqNpVpEMB$26.HpBpLpPpIqGrOrFrPqWpS9.tJ8.CpGpNpLpOpTqBqCpSpIQG$26.pCpMpAMA.qBrNrSsKrX10.rF10.NUWVSME$25.RpLV4.pIrPtDtSuAtH9.qP$25.pFpM6.rStTuUvNvLuP8.HpQ$25.pLpE6.pXuEvWwIxDwVtA8.qE$24.qUpOpE7.uFwIxSxVyKyBqQ7.pT$25.pHpQ7.pEwPyHxWyN2yOT6.pNE$25.sQqL8.vG2yOwGxVyOvQ6.pME$12.MXpEpDVDrK7.vI8.J2yOwBvRxXyJsO4.OpN$5.HVpSpB.IpBpMpNpGpEpPpVpFpV6.pCpJ8.tVyOxLvNwFxLvLqXNCLpEpN$5.PqKqRrCqCpJpQpEJ3.KpXrV7.wD8.ExAyLwIvPwEvRtDqOpLpMpUX$5.JqWsEsTsGqKpDB6.pOvX7.sG8.rPwUwSvPvFuTtMrSqOqDpSK$5.DrBtAtJtHrCE8.pIsA6.rEF7.MtJvOvFuHtOsTrSqWqDpA$4.NpGqNsUtPsOrEX9.sT7.rW8.qUtKtWtGsLrRrBqHpIH$3.SpO2pQqUsBrSrLqOpH9.uQ7.qVA6.pSrQsJsDrKqQqApFK$2.OpNpLTH.qGrPrSsDrS9.DqF6.BqGV4.NpQqTrIrBqJpOWH$2.pFpMJ3.pKrOsXtJtNsW9.rK8.pKpOpDSXpJpW2qKpUpFMD$2.pQT5.rUtPuHvEvBtD8.LpL8.EWpKpPpQpRpNpDOF$A.pWG5.rCuDvRwCwVwKqP8.qI12.CEB$2.qB7.uIwKxK2yBxMB7.pU$.sJqDB6.rAwVyExW2yOwJ7.pNF$2.rLM7.wVyOyIwX2yOtB6.pLH$2.sHpR7.qT2yOwBwKyOxKqJ4.IpKC$3.vW8.wOyOwUvMwRxPuCpJ2.CXpN$3.WE7.qLyNxWvVvSwMvJsApPVpEpQX$4.vA8.tVxLwKvNvKuWtDrFqGpXpQK$5.rK7.pXuUvVvEuLtUsWrRqUqCpA$5.qBpA6.KrXuDuEtKsPrXrEqHpJH$6.qNS5.GqKsJsTsHrMqTqBpGL$7.pXpF4.SqErIrNrDqIpOXJ$8.pEpQpBWpCpOqFqNqLpVpFOF$9.EpCpOpPpQpSpQpGQI$12.F2LIC!",
    },
];

#[cfg(test)]
mod tests {
    use super::{curated_species, decode_rle_2d, parse_shell_weights};
    use crate::lenia::{GrowthFuncType, KernelMode};

    #[test]
    fn parse_shell_weights_supports_fractions() {
        let weights = parse_shell_weights("5/12,1,1/2").expect("weights");
        assert_eq!(weights.len(), 3);
        assert!((weights[0] - 5.0 / 12.0).abs() < 1e-9);
        assert!((weights[2] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn decode_rle_2d_pads_rows() {
        let pattern = decode_rle_2d("2A$B!").expect("pattern");
        assert_eq!(pattern.nrows(), 2);
        assert_eq!(pattern.ncols(), 2);
        assert!(pattern[(0, 0)] > 0.0);
        assert_eq!(pattern[(1, 1)], 0.0);
    }

    #[test]
    fn curated_species_loads_into_lenia_band_mode() {
        let loaded = curated_species()[0].load().expect("species");
        assert_eq!(loaded.params.kernel_mode, KernelMode::LeniaBands);
        assert_eq!(loaded.params.growth_func_type, GrowthFuncType::Polynomial);
        assert!(loaded.world.sum() > 0.0);
        assert!(loaded.world.nrows() >= loaded.pattern_rows);
    }
}
