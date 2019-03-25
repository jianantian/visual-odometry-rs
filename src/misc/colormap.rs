// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Colormap helpers for visualizations.

/// Viridis colormap in an RGB u8 vector.
pub fn viridis_u8() -> Vec<(u8, u8, u8)> {
    viridis()
        .into_iter()
        .map(|(r, g, b)| (to_u8(r), to_u8(g), to_u8(b)))
        .collect()
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn to_u8(x: f32) -> u8 {
    (255.0 * x).round() as u8
}

/// Viridis colormap in an RGB f32 ([0-1]) vector.
pub fn viridis() -> Vec<(f32, f32, f32)> {
    vec![
        (0.267_004, 0.004_874, 0.329_415),
        (0.268_510, 0.009_604, 0.335_426),
        (0.269_943, 0.014_624, 0.341_378),
        (0.271_304, 0.019_941, 0.347_268),
        (0.272_593, 0.025_563, 0.353_093),
        (0.273_809, 0.031_497, 0.358_852),
        (0.274_952, 0.037_751, 0.364_543),
        (0.276_022, 0.044_167, 0.370_164),
        (0.277_018, 0.050_344, 0.375_714),
        (0.277_941, 0.056_324, 0.381_190),
        (0.278_790, 0.062_145, 0.386_592),
        (0.279_565, 0.067_835, 0.391_917),
        (0.280_266, 0.073_417, 0.397_163),
        (0.280_893, 0.078_907, 0.402_329),
        (0.281_445, 0.084_319, 0.407_414),
        (0.281_923, 0.089_666, 0.412_415),
        (0.282_327, 0.094_955, 0.417_330),
        (0.282_656, 0.100_195, 0.422_160),
        (0.282_910, 0.105_393, 0.426_902),
        (0.283_090, 0.110_553, 0.431_553),
        (0.283_197, 0.115_679, 0.436_114),
        (0.283_228, 0.120_777, 0.440_584),
        (0.283_186, 0.125_847, 0.444_960),
        (0.283_072, 0.130_894, 0.449_241),
        (0.282_883, 0.135_920, 0.453_427),
        (0.282_622, 0.140_925, 0.457_517),
        (0.282_290, 0.145_912, 0.461_509),
        (0.281_886, 0.150_881, 0.465_404),
        (0.281_412, 0.155_834, 0.469_201),
        (0.280_867, 0.160_771, 0.472_899),
        (0.280_254, 0.165_692, 0.476_497),
        (0.279_573, 0.170_598, 0.479_996),
        (0.278_826, 0.175_490, 0.483_396),
        (0.278_012, 0.180_366, 0.486_697),
        (0.277_134, 0.185_228, 0.489_898),
        (0.276_193, 0.190_074, 0.493_000),
        (0.275_191, 0.194_905, 0.496_004),
        (0.274_128, 0.199_720, 0.498_911),
        (0.273_005, 0.204_520, 0.501_720),
        (0.271_828, 0.209_303, 0.504_434),
        (0.270_594, 0.214_068, 0.507_052),
        (0.269_307, 0.218_817, 0.509_576),
        (0.267_968, 0.223_549, 0.512_008),
        (0.266_579, 0.228_262, 0.514_348),
        (0.265_144, 0.232_955, 0.516_599),
        (0.263_663, 0.237_630, 0.518_761),
        (0.262_138, 0.242_286, 0.520_837),
        (0.260_571, 0.246_921, 0.522_828),
        (0.258_964, 0.251_536, 0.524_736),
        (0.257_322, 0.256_130, 0.526_563),
        (0.255_645, 0.260_702, 0.528_311),
        (0.253_934, 0.265_253, 0.529_982),
        (0.252_194, 0.269_783, 0.531_579),
        (0.250_424, 0.274_290, 0.533_102),
        (0.248_628, 0.278_775, 0.534_555),
        (0.246_811, 0.283_236, 0.535_940),
        (0.244_972, 0.287_675, 0.537_260),
        (0.243_113, 0.292_091, 0.538_515),
        (0.241_237, 0.296_484, 0.539_709),
        (0.239_345, 0.300_854, 0.540_843),
        (0.237_441, 0.305_202, 0.541_921),
        (0.235_526, 0.309_526, 0.542_943),
        (0.233_602, 0.313_827, 0.543_914),
        (0.231_673, 0.318_105, 0.544_834),
        (0.229_739, 0.322_361, 0.545_706),
        (0.227_801, 0.326_594, 0.546_532),
        (0.225_863, 0.330_805, 0.547_313),
        (0.223_925, 0.334_994, 0.548_052),
        (0.221_989, 0.339_161, 0.548_752),
        (0.220_056, 0.343_306, 0.549_413),
        (0.218_129, 0.347_431, 0.550_037),
        (0.216_209, 0.351_535, 0.550_627),
        (0.214_297, 0.355_619, 0.551_184),
        (0.212_394, 0.359_682, 0.551_710),
        (0.210_503, 0.363_726, 0.552_206),
        (0.208_623, 0.367_751, 0.552_674),
        (0.206_756, 0.371_757, 0.553_116),
        (0.204_902, 0.375_745, 0.553_532),
        (0.203_063, 0.379_716, 0.553_925),
        (0.201_238, 0.383_669, 0.554_294),
        (0.199_429, 0.387_606, 0.554_642),
        (0.197_636, 0.391_527, 0.554_969),
        (0.195_859, 0.395_432, 0.555_276),
        (0.194_100, 0.399_323, 0.555_564),
        (0.192_357, 0.403_199, 0.555_835),
        (0.190_631, 0.407_061, 0.556_089),
        (0.188_922, 0.410_910, 0.556_326),
        (0.187_230, 0.414_746, 0.556_547),
        (0.185_555, 0.418_570, 0.556_752),
        (0.183_897, 0.422_382, 0.556_943),
        (0.182_255, 0.426_184, 0.557_120),
        (0.180_629, 0.429_974, 0.557_282),
        (0.179_018, 0.433_755, 0.557_430),
        (0.177_422, 0.437_527, 0.557_564),
        (0.175_841, 0.441_289, 0.557_685),
        (0.174_273, 0.445_044, 0.557_792),
        (0.172_718, 0.448_790, 0.557_885),
        (0.171_176, 0.452_529, 0.557_964),
        (0.169_645, 0.456_262, 0.558_030),
        (0.168_126, 0.459_988, 0.558_081),
        (0.166_617, 0.463_708, 0.558_119),
        (0.165_117, 0.467_422, 0.558_141),
        (0.163_625, 0.471_132, 0.558_148),
        (0.162_141, 0.474_838, 0.558_139),
        (0.160_664, 0.478_539, 0.558_114),
        (0.159_194, 0.482_237, 0.558_072),
        (0.157_729, 0.485_931, 0.558_013),
        (0.156_269, 0.489_623, 0.557_936),
        (0.154_814, 0.493_312, 0.557_839),
        (0.153_364, 0.497_000, 0.557_723),
        (0.151_918, 0.500_685, 0.557_587),
        (0.150_476, 0.504_369, 0.557_429),
        (0.149_039, 0.508_051, 0.557_250),
        (0.147_607, 0.511_732, 0.557_048),
        (0.146_180, 0.515_413, 0.556_822),
        (0.144_758, 0.519_093, 0.556_571),
        (0.143_343, 0.522_772, 0.556_294),
        (0.141_935, 0.526_452, 0.555_990),
        (0.140_535, 0.530_132, 0.555_658),
        (0.139_147, 0.533_812, 0.555_297),
        (0.137_770, 0.537_492, 0.554_906),
        (0.136_408, 0.541_172, 0.554_483),
        (0.135_065, 0.544_853, 0.554_029),
        (0.133_742, 0.548_534, 0.553_541),
        (0.132_444, 0.552_216, 0.553_018),
        (0.131_172, 0.555_898, 0.552_459),
        (0.129_932, 0.559_581, 0.551_863),
        (0.128_729, 0.563_265, 0.551_229),
        (0.127_567, 0.566_948, 0.550_555),
        (0.126_453, 0.570_633, 0.549_841),
        (0.125_393, 0.574_317, 0.549_085),
        (0.124_394, 0.578_002, 0.548_287),
        (0.123_462, 0.581_686, 0.547_444),
        (0.122_605, 0.585_371, 0.546_557),
        (0.121_831, 0.589_055, 0.545_622),
        (0.121_148, 0.592_738, 0.544_641),
        (0.120_565, 0.596_421, 0.543_610),
        (0.120_091, 0.600_103, 0.542_530),
        (0.119_737, 0.603_784, 0.541_399),
        (0.119_511, 0.607_463, 0.540_217),
        (0.119_423, 0.611_141, 0.538_981),
        (0.119_482, 0.614_817, 0.537_692),
        (0.119_698, 0.618_490, 0.536_347),
        (0.120_080, 0.622_160, 0.534_946),
        (0.120_638, 0.625_828, 0.533_488),
        (0.121_379, 0.629_492, 0.531_972),
        (0.122_312, 0.633_152, 0.530_398),
        (0.123_443, 0.636_808, 0.528_763),
        (0.124_779, 0.640_460, 0.527_067),
        (0.126_325, 0.644_107, 0.525_310),
        (0.128_087, 0.647_748, 0.523_490),
        (0.130_066, 0.651_384, 0.521_607),
        (0.132_267, 0.655_013, 0.519_660),
        (0.134_691, 0.658_636, 0.517_648),
        (0.137_339, 0.662_251, 0.515_571),
        (0.140_209, 0.665_859, 0.513_426),
        (0.143_302, 0.669_458, 0.511_215),
        (0.146_616, 0.673_049, 0.508_936),
        (0.150_147, 0.676_631, 0.506_588),
        (0.153_894, 0.680_203, 0.504_172),
        (0.157_851, 0.683_765, 0.501_685),
        (0.162_015, 0.687_316, 0.499_129),
        (0.166_383, 0.690_856, 0.496_501),
        (0.170_948, 0.694_384, 0.493_802),
        (0.175_706, 0.697_899, 0.491_032),
        (0.180_653, 0.701_402, 0.488_189),
        (0.185_782, 0.704_891, 0.485_273),
        (0.191_090, 0.708_366, 0.482_283),
        (0.196_570, 0.711_826, 0.479_221),
        (0.202_219, 0.715_271, 0.476_084),
        (0.208_030, 0.718_700, 0.472_873),
        (0.214_000, 0.722_113, 0.469_587),
        (0.220_123, 0.725_509, 0.466_226),
        (0.226_396, 0.728_887, 0.462_789),
        (0.232_814, 0.732_247, 0.459_276),
        (0.239_373, 0.735_588, 0.455_688),
        (0.246_069, 0.738_909, 0.452_024),
        (0.252_898, 0.742_211, 0.448_283),
        (0.259_856, 0.745_491, 0.444_466),
        (0.266_941, 0.748_750, 0.440_572),
        (0.274_149, 0.751_988, 0.436_600),
        (0.281_476, 0.755_202, 0.432_552),
        (0.288_921, 0.758_393, 0.428_426),
        (0.296_478, 0.761_561, 0.424_223),
        (0.304_147, 0.764_704, 0.419_943),
        (0.311_925, 0.767_822, 0.415_586),
        (0.319_808, 0.770_914, 0.411_152),
        (0.327_795, 0.773_979, 0.406_640),
        (0.335_885, 0.777_017, 0.402_049),
        (0.344_074, 0.780_028, 0.397_381),
        (0.352_359, 0.783_010, 0.392_635),
        (0.360_740, 0.785_964, 0.387_813),
        (0.369_214, 0.788_887, 0.382_914),
        (0.377_778, 0.791_781, 0.377_938),
        (0.386_432, 0.794_644, 0.372_886),
        (0.395_174, 0.797_475, 0.367_757),
        (0.404_001, 0.800_274, 0.362_552),
        (0.412_913, 0.803_040, 0.357_268),
        (0.421_908, 0.805_774, 0.351_910),
        (0.430_983, 0.808_473, 0.346_476),
        (0.440_136, 0.811_138, 0.340_967),
        (0.449_367, 0.813_768, 0.335_384),
        (0.458_673, 0.816_362, 0.329_727),
        (0.468_053, 0.818_921, 0.323_997),
        (0.477_504, 0.821_443, 0.318_195),
        (0.487_025, 0.823_928, 0.312_321),
        (0.496_615, 0.826_376, 0.306_376),
        (0.506_271, 0.828_786, 0.300_362),
        (0.515_991, 0.831_157, 0.294_278),
        (0.525_776, 0.833_490, 0.288_126),
        (0.535_621, 0.835_784, 0.281_908),
        (0.545_524, 0.838_039, 0.275_626),
        (0.555_483, 0.840_254, 0.269_281),
        (0.565_497, 0.842_429, 0.262_876),
        (0.575_562, 0.844_565, 0.256_414),
        (0.585_677, 0.846_661, 0.249_897),
        (0.595_839, 0.848_717, 0.243_328),
        (0.606_045, 0.850_733, 0.236_712),
        (0.616_292, 0.852_709, 0.230_051),
        (0.626_579, 0.854_645, 0.223_352),
        (0.636_901, 0.856_542, 0.216_620),
        (0.647_256, 0.858_399, 0.209_860),
        (0.657_641, 0.860_218, 0.203_082),
        (0.668_053, 0.861_999, 0.196_293),
        (0.678_488, 0.863_742, 0.189_503),
        (0.688_943, 0.865_447, 0.182_724),
        (0.699_414, 0.867_117, 0.175_970),
        (0.709_898, 0.868_750, 0.169_257),
        (0.720_391, 0.870_350, 0.162_602),
        (0.730_889, 0.871_915, 0.156_028),
        (0.741_388, 0.873_449, 0.149_561),
        (0.751_884, 0.874_951, 0.143_228),
        (0.762_373, 0.876_423, 0.137_064),
        (0.772_851, 0.877_868, 0.131_108),
        (0.783_315, 0.879_285, 0.125_405),
        (0.793_759, 0.880_677, 0.120_005),
        (0.804_181, 0.882_046, 0.114_965),
        (0.814_576, 0.883_393, 0.110_346),
        (0.824_940, 0.884_720, 0.106_217),
        (0.835_269, 0.886_029, 0.102_645),
        (0.845_560, 0.887_322, 0.099_702),
        (0.855_809, 0.888_601, 0.097_451),
        (0.866_013, 0.889_868, 0.095_952),
        (0.876_168, 0.891_124, 0.095_250),
        (0.886_271, 0.892_373, 0.095_374),
        (0.896_320, 0.893_616, 0.096_335),
        (0.906_311, 0.894_854, 0.098_124),
        (0.916_242, 0.896_091, 0.100_716),
        (0.926_105, 0.897_329, 0.104_070),
        (0.935_904, 0.898_570, 0.108_130),
        (0.945_636, 0.899_815, 0.112_837),
        (0.955_299, 0.901_065, 0.118_128),
        (0.964_893, 0.902_323, 0.123_940),
        (0.974_416, 0.903_589, 0.130_214),
        (0.983_868, 0.904_867, 0.136_896),
        (0.993_247, 0.906_156, 0.143_936),
    ]
}
