use ofx::*;

plugin_module!(
	"net.itadinanta.ofx-rs.simple_plugin_1",
	ApiVersion(1),
	PluginVersion(1, 0),
	SimplePlugin::new
);

#[derive(Default)]
struct SimplePlugin {
	host_supports_multiple_clip_depths: Bool,
}

impl SimplePlugin {
	pub fn new() -> SimplePlugin {
		SimplePlugin::default()
	}
}
#[allow(unused)]
struct MyInstanceData {
	is_general_effect: bool,

	source_clip: ImageClipHandle,
	mask_clip: Option<ImageClipHandle>,
	output_clip: ImageClipHandle,

	scale_param: ParamHandle<Double>,

	per_component_scale_param: ParamHandle<Bool>,

	scale_r_param: ParamHandle<Double>,
	scale_g_param: ParamHandle<Double>,
	scale_b_param: ParamHandle<Double>,
	scale_a_param: ParamHandle<Double>,
}
const PARAM_MAIN_NAME: &str = "Main";
const PARAM_SCALE_NAME: &str = "scale";
const PARAM_SCALE_R_NAME: &str = "scaleR";
const PARAM_SCALE_G_NAME: &str = "scaleG";
const PARAM_SCALE_B_NAME: &str = "scaleB";
const PARAM_SCALE_A_NAME: &str = "scaleA";
const PARAM_SCALE_COMPONENTS_NAME: &str = "scaleComponents";
const PARAM_COMPONENT_SCALES_NAME: &str = "componentScales";

impl Execute for SimplePlugin {
	#[allow(clippy::float_cmp)]
	fn execute(&mut self, plugin_context: &PluginContext, action: &mut Action) -> Result<Int> {
		use Action::*;
		match *action {
			Render(ref mut _effect, ref _in_args) => OK,

			IsIdentity(ref mut effect, ref in_args, ref mut out_args) => {
				let time = in_args.get_time()?;
				let _render_window = in_args.get_render_window()?;
				let instance_data: &MyInstanceData = effect.get_instance_data()?;

				let scale_value = instance_data.scale_param.get_value_at_time(time)?;

				let (sr, sg, sb, sa) = if instance_data.source_clip.get_components()?.is_rgb() {
					(
						instance_data.scale_r_param.get_value_at_time(time)?,
						instance_data.scale_g_param.get_value_at_time(time)?,
						instance_data.scale_b_param.get_value_at_time(time)?,
						instance_data.scale_a_param.get_value_at_time(time)?,
					)
				} else {
					(1., 1., 1., 1.)
				};
				if scale_value == 1. && sr == 1. && sg == 1. && sb == 1. && sa == 1. {
					out_args.set_name(&image_effect_simple_source_clip_name())?;
					OK
				} else {
					REPLY_DEFAULT
				}
			}

			InstanceChanged(ref mut effect, ref in_args) => {
				if in_args.get_change_reason()? == Change::UserEdited {
					let obj_changed = in_args.get_name()?;
					let expected = match in_args.get_type()? {
						Type::Clip => Some(image_effect_simple_source_clip_name()),
						Type::Parameter => Some(PARAM_SCALE_COMPONENTS_NAME.to_owned()),
						_ => None,
					};

					if expected == Some(obj_changed) {
						Self::set_per_component_scale_enabledness(effect)?;
						OK
					} else {
						REPLY_DEFAULT
					}
				} else {
					REPLY_DEFAULT
				}
			}

			GetRegionOfDefinition(ref mut effect, ref in_args, ref mut out_args) => {
				let time = in_args.get_time()?;
				let rod = effect
					.get_instance_data::<MyInstanceData>()?
					.source_clip
					.get_region_of_definition(time)?;
				out_args.set_region_of_definition(rod)?;

				OK
			}

			GetRegionsOfInterest(ref mut effect, ref in_args, ref mut out_args) => {
				let roi = in_args.get_region_of_interest()?;

				out_args.set_raw(image_clip_prop_roi!(clip_source!()), &roi)?;

				if effect
					.get_instance_data::<MyInstanceData>()?
					.is_general_effect
					&& effect.get_clip(clip_mask!())?.get_connected()?
				{
					out_args.set_raw(image_clip_prop_roi!(clip_mask!()), &roi)?;
				}

				OK
			}

			GetTimeDomain(ref mut effect, ref mut out_args) => {
				let my_data: &MyInstanceData = effect.get_instance_data()?;
				let frame_range = my_data.source_clip.get_frame_range()?;
				out_args.set_frame_range(frame_range)?;

				OK
			}

			GetClipPreferences(ref mut effect, ref mut out_args) => {
				let my_data: &MyInstanceData = effect.get_instance_data()?;
				let bit_depth = my_data.source_clip.get_pixel_depth()?;
				let image_component = my_data.source_clip.get_components()?;
				let output_component = match image_component {
					ImageComponent::RGBA | ImageComponent::RGB => ImageComponent::RGBA,
					_ => ImageComponent::Alpha,
				};
				out_args.set_raw(
					image_clip_prop_components!(clip_output!()),
					output_component.to_bytes(),
				)?;

				if self.host_supports_multiple_clip_depths {
					out_args
						.set_raw(image_clip_prop_depth!(clip_output!()), bit_depth.to_bytes())?;
				}

				if my_data.is_general_effect {
					let is_mask_connected = my_data
						.mask_clip
						.as_ref()
						.and_then(|mask| mask.get_connected().ok())
						.unwrap_or_default();

					if is_mask_connected {
						out_args.set_raw(
							image_clip_prop_components!(clip_mask!()),
							ImageComponent::Alpha.to_bytes(),
						)?;
						if self.host_supports_multiple_clip_depths {
							out_args.set_raw(
								image_clip_prop_depth!(clip_mask!()),
								bit_depth.to_bytes(),
							)?;
						}
					}
				}

				OK
			}

			CreateInstance(ref mut effect) => {
				let mut effect_props = effect.properties()?;
				let mut param_set = effect.parameter_set()?;

				let is_general_effect = effect_props.get_context()?.is_general();
				let per_component_scale_param = param_set.parameter(PARAM_SCALE_COMPONENTS_NAME)?;

				let source_clip = effect.get_simple_input_clip()?;
				let output_clip = effect.get_output_clip()?;
				let mask_clip = if is_general_effect {
					Some(effect.get_clip(clip_mask!())?)
				} else {
					None
				};

				let scale_param = param_set.parameter(PARAM_SCALE_NAME)?;
				let scale_r_param = param_set.parameter(PARAM_SCALE_R_NAME)?;
				let scale_g_param = param_set.parameter(PARAM_SCALE_G_NAME)?;
				let scale_b_param = param_set.parameter(PARAM_SCALE_B_NAME)?;
				let scale_a_param = param_set.parameter(PARAM_SCALE_A_NAME)?;

				effect.set_instance_data(MyInstanceData {
					is_general_effect,
					source_clip,
					mask_clip,
					output_clip,
					per_component_scale_param,
					scale_param,
					scale_r_param,
					scale_g_param,
					scale_b_param,
					scale_a_param,
				})?;

				Self::set_per_component_scale_enabledness(effect)?;

				OK
			}

			DestroyInstance(ref mut _effect) => OK,

			DescribeInContext(ref mut effect, ref in_args) => {
				let mut output_clip = effect.new_output_clip()?;
				output_clip
					.set_supported_components(&[ImageComponent::RGBA, ImageComponent::Alpha])?;

				let mut input_clip = effect.new_simple_input_clip()?;
				input_clip
					.set_supported_components(&[ImageComponent::RGBA, ImageComponent::Alpha])?;

				if in_args.get_context()?.is_general() {
					let mut mask = effect.new_clip(clip_mask!())?;
					mask.set_supported_components(&[ImageComponent::Alpha])?;
					mask.set_optional(true)?;
				}

				fn define_scale_param(
					param_set: &mut ParamSetHandle,
					name: &str,
					label: &'static str,
					script_name: &'static str,
					hint: &'static str,
					parent: Option<&'static str>,
				) -> Result<()> {
					let mut param_props = param_set.param_define_double(name)?;

					param_props.set_double_type(ParamDoubleType::Scale)?;
					param_props.set_label(label)?;
					param_props.set_default(1.0)?;
					param_props.set_display_min(1.0)?;
					param_props.set_display_min(1.0)?;
					param_props.set_display_max(100.0)?;
					param_props.set_hint(hint)?;
					param_props.set_script_name(script_name)?;

					if let Some(parent) = parent {
						param_props.set_parent(parent)?;
					}

					Ok(())
				}

				let mut param_set = effect.parameter_set()?;
				define_scale_param(
					&mut param_set,
					PARAM_SCALE_NAME,
					"scale",
					PARAM_SCALE_NAME,
					"Scales all component in the image",
					None,
				)?;

				let mut param_props =
					param_set.param_define_boolean(PARAM_SCALE_COMPONENTS_NAME)?;
				param_props.set_default(false)?;
				param_props.set_hint("Enables scale on individual components")?;
				param_props.set_script_name(PARAM_SCALE_COMPONENTS_NAME)?;
				param_props.set_label("Scale Individual Components")?;

				let mut param_props = param_set.param_define_group(PARAM_COMPONENT_SCALES_NAME)?;
				param_props.set_hint("Scales on the individual component")?;
				param_props.set_label("Components")?;

				define_scale_param(
					&mut param_set,
					PARAM_SCALE_R_NAME,
					"red",
					PARAM_SCALE_R_NAME,
					"Scales the red component of the image",
					Some(PARAM_COMPONENT_SCALES_NAME),
				)?;
				define_scale_param(
					&mut param_set,
					PARAM_SCALE_G_NAME,
					"green",
					PARAM_SCALE_G_NAME,
					"Scales the green component of the image",
					Some(PARAM_COMPONENT_SCALES_NAME),
				)?;
				define_scale_param(
					&mut param_set,
					PARAM_SCALE_B_NAME,
					"blue",
					PARAM_SCALE_B_NAME,
					"Scales the blue component of the image",
					Some(PARAM_COMPONENT_SCALES_NAME),
				)?;
				define_scale_param(
					&mut param_set,
					PARAM_SCALE_A_NAME,
					"alpha",
					PARAM_SCALE_A_NAME,
					"Scales the alpha component of the image",
					Some(PARAM_COMPONENT_SCALES_NAME),
				)?;

				let mut param_props = param_set.param_define_page(PARAM_MAIN_NAME)?;
				param_props.set_children(&[
					PARAM_SCALE_NAME,
					PARAM_SCALE_COMPONENTS_NAME,
					PARAM_SCALE_R_NAME,
					PARAM_SCALE_G_NAME,
					PARAM_SCALE_B_NAME,
					PARAM_SCALE_A_NAME,
				])?;

				OK
			}

			Describe(ref mut effect) => {
				self.host_supports_multiple_clip_depths = plugin_context
					.get_host()
					.get_supports_multiple_clip_depths()?;

				let mut effect_properties = effect.properties()?;
				effect_properties.set_grouping("Ofx-rs")?;

				effect_properties.set_label("Ofx-rs simple_plugin sample")?;
				effect_properties.set_short_label("Ofx-rs simple_plugin")?;
				effect_properties.set_long_label("Ofx-rs simple_plugin in examples")?;

				effect_properties.set_supported_pixel_depths(&[
					BitDepth::Byte,
					BitDepth::Short,
					BitDepth::Float,
				])?;
				effect_properties.set_supported_contexts(&[
					ImageEffectContext::Filter,
					ImageEffectContext::General,
				])?;

				OK
			}

			_ => REPLY_DEFAULT,
		}
	}
}

impl SimplePlugin {
	fn set_per_component_scale_enabledness(effect: &mut ImageEffectHandle) -> Result<()> {
		let instance_data: &mut MyInstanceData = effect.get_instance_data()?;
		let input_clip = effect.get_simple_input_clip()?;
		let is_input_rgb = input_clip.get_connected()? && input_clip.get_components()?.is_rgb();
		instance_data
			.per_component_scale_param
			.set_enabled(is_input_rgb)?;
		let per_component_scale =
			is_input_rgb && instance_data.per_component_scale_param.get_value()?;
		for scale_param in &mut [
			&mut instance_data.scale_r_param,
			&mut instance_data.scale_g_param,
			&mut instance_data.scale_b_param,
			&mut instance_data.scale_a_param,
		] {
			scale_param.set_enabled(per_component_scale)?;
			instance_data
				.scale_param
				.set_enabled(!per_component_scale)?
		}

		Ok(())
	}
}
